use libxml::{
    tree::{Document, Node, NodeType},
    xpath::Context,
};
use std::fmt::Write;
use std::{collections::HashSet, fs::DirEntry};

use crate::{
    constants::{self, NEGATIVE_LEAD_IMAGE_URL_HINTS_REGEX},
    full_text_parser::error::FullTextParserError,
    image_object::ImageObject,
    video_object::VideoObject,
};

pub struct Util;

impl Util {
    pub fn serialize_node(doc: &Document, node: &Node) -> String {
        doc.node_to_string(node)
    }

    pub fn check_extension(path: &DirEntry, extension: &str) -> bool {
        if let Some(ext) = path.path().extension() {
            ext.to_str() == Some(extension)
        } else {
            false
        }
    }

    pub fn str_extract_value<'a>(identifier: &str, line: &'a str) -> &'a str {
        let value = &line[identifier.len()..];
        let value = value.trim();
        match value.find('#') {
            Some(pos) => &value[..pos],
            None => value,
        }
    }

    pub fn split_values(values: &str) -> Vec<&str> {
        values.split('|').map(|s| s.trim()).collect()
    }

    pub fn evaluate_xpath(
        xpath_ctx: &Context,
        xpath: &str,
        thorw_if_empty: bool,
    ) -> Result<Vec<Node>, FullTextParserError> {
        let res = xpath_ctx.evaluate(xpath).map_err(|()| {
            log::debug!("Evaluation of xpath '{}' yielded no results", xpath);
            FullTextParserError::Xml
        })?;

        let node_vec = res.get_nodes_as_vec();

        if node_vec.is_empty() {
            log::debug!("Evaluation of xpath '{}' yielded no results", xpath);
            if thorw_if_empty {
                return Err(FullTextParserError::Xml);
            }
        }

        Ok(node_vec)
    }

    pub fn get_attribute(
        context: &Context,
        xpath: &str,
        attribute: &str,
    ) -> Result<String, FullTextParserError> {
        Util::evaluate_xpath(context, xpath, false)?
            .iter()
            .find_map(|node| node.get_attribute(attribute))
            .ok_or(FullTextParserError::Xml)
    }

    pub fn extract_value(context: &Context, xpath: &str) -> Result<String, FullTextParserError> {
        let node_vec = Util::evaluate_xpath(context, xpath, false)?;
        if let Some(val) = node_vec.first() {
            return Ok(val.get_content());
        }

        Err(FullTextParserError::Xml)
    }

    pub fn extract_value_merge(
        context: &Context,
        xpath: &str,
    ) -> Result<String, FullTextParserError> {
        let node_vec = Util::evaluate_xpath(context, xpath, true)?;
        let mut val = String::new();
        for node in node_vec {
            let part =
                node.get_content()
                    .split_whitespace()
                    .fold(String::new(), |mut output, s| {
                        let _ = write!(output, " {s}");
                        output
                    });
            val.push_str(&part);
            val.push(' ');
        }

        Ok(val.trim().to_string())
    }

    pub fn strip_node(context: &Context, xpath: &str) -> Result<(), FullTextParserError> {
        let node_vec = Util::evaluate_xpath(context, xpath, false)?;
        let node_vec_clone = node_vec.clone();

        for mut node in node_vec {
            if node.is_null() {
                continue;
            }

            let tag_name = node.get_name();
            if constants::EMBED_TAG_NAMES.contains(tag_name.to_uppercase().as_str())
                && node
                    .get_attributes()
                    .iter()
                    .any(|(_name, value)| constants::VIDEOS.is_match(value))
            {
                continue;
            }

            if Self::parent_part_of_result(&node, &node_vec_clone) {
                continue;
            }

            node.unlink();
        }
        Ok(())
    }

    fn parent_part_of_result(node: &Node, xpath_result: &[Node]) -> bool {
        if let Some(parent) = node.get_parent() {
            for n in xpath_result {
                if n == &parent {
                    return true;
                }
            }

            return Self::parent_part_of_result(&parent, xpath_result);
        }

        false
    }

    pub fn strip_id_or_class(
        context: &Context,
        id_or_class: &str,
    ) -> Result<(), FullTextParserError> {
        let xpath = &format!(
            "//*[contains(@class, '{}') or contains(@id, '{}')]",
            id_or_class, id_or_class
        );

        let mut ancestor = xpath.clone();
        if ancestor.starts_with("//") {
            ancestor = ancestor.chars().skip(2).collect();
        }

        let query = &format!("{}[not(ancestor::{})]", xpath, ancestor);
        let node_vec = Util::evaluate_xpath(context, query, false)?;
        for mut node in node_vec {
            if node.is_null() {
                continue;
            }

            node.unlink();
        }
        Ok(())
    }

    pub fn get_signature(node: &Node) -> String {
        let match_string = node
            .get_class_names()
            .iter()
            .fold(String::new(), |a, b| format!("{a} {b}"));
        match node.get_property("id") {
            Some(id) => format!("{match_string} {id}"),
            None => match_string,
        }
    }

    pub fn is_probably_visible(node: &Node) -> bool {
        let is_hidden = node.has_attribute("hidden");
        let aria_hidden = node
            .get_attribute("aria-hidden")
            .map(|attr| attr == "true")
            .unwrap_or(false);
        let has_fallback_image = node
            .get_class_names()
            .iter()
            .any(|class| class.contains("fallback-image"));

        !is_hidden && !aria_hidden || has_fallback_image
    }

    pub fn is_whitespace(node: &Node) -> bool {
        let content = node.get_content();
        let tag_name = node.get_name().to_uppercase();

        let is_text_node = node
            .get_type()
            .map(|t| t == NodeType::TextNode)
            .unwrap_or(false);
        let is_element_node = node
            .get_type()
            .map(|t| t == NodeType::ElementNode)
            .unwrap_or(false);

        (is_text_node && content.trim().is_empty()) || (is_element_node && tag_name == "BR")
    }

    pub fn remove_and_next(node: &mut Node) -> Option<Node> {
        if node.is_null() {
            return None;
        }

        let next_node = Self::next_node(node, true);
        node.unlink();
        next_node
    }

    pub fn next_node(node: &Node, ignore_self_and_kids: bool) -> Option<Node> {
        let mut node = node.clone();

        // First check for kids if those aren't being ignored
        let first_child = node.get_first_child();
        if !ignore_self_and_kids && first_child.is_some() {
            return first_child;
        }

        // Then for siblings...
        let next_sibling = node.get_next_sibling();
        if next_sibling.is_some() {
            return next_sibling;
        }

        // And finally, move up the parent chain *and* find a sibling
        // (because this is depth-first traversal, we will have already
        // seen the parent nodes themselves).
        while let Some(parent) = node.get_parent() {
            let parent_name = parent.get_name().to_uppercase();
            if parent_name == "HTML" {
                break;
            }

            let next_sibling = parent.get_next_sibling();
            if next_sibling.is_some() {
                return next_sibling;
            } else {
                node = parent;
            }
        }

        None
    }

    pub fn get_inner_text(node: &Node, normalize_spaces: bool) -> String {
        let content = node.get_content().trim().to_owned();
        if normalize_spaces {
            constants::NORMALIZE.replace_all(&content, " ").into()
        } else {
            content
        }
    }

    pub fn text_similarity(a: &str, b: &str) -> f64 {
        let a = a.to_lowercase();
        let b = b.to_lowercase();
        let tokens_a = constants::TOKENIZE
            .split(&a)
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();
        let tokens_b = constants::TOKENIZE
            .split(&b)
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();
        if tokens_a.is_empty() || tokens_b.is_empty() {
            return 0.0;
        }

        let tokens_b_total = tokens_b.join(" ").len() as f64;
        let uniq_tokens_b = tokens_b
            .into_iter()
            .filter(|token| !tokens_a.iter().any(|t| t == token))
            .collect::<Vec<_>>();
        let uniq_tokens_b_total = uniq_tokens_b.join(" ").len() as f64;

        let distance_b = uniq_tokens_b_total / tokens_b_total;
        1.0 - distance_b
    }

    // Check if this node is an H1 or H2 element whose content is mostly
    // the same as the article title.
    pub fn header_duplicates_title(node: &Node, title: Option<&str>) -> bool {
        let name = node.get_name().to_lowercase();
        if name != "h1" && name != "h2" {
            return false;
        }
        let heading = Util::get_inner_text(node, false);

        if let Some(title) = title {
            Util::text_similarity(title, &heading) > 0.75
        } else {
            false
        }
    }

    pub fn has_any_descendent_tag(node: &Node, tag_names: &HashSet<&str>) -> bool {
        let children = node.get_child_elements();
        let is_direct_child = children
            .iter()
            .map(|node| node.get_name().to_uppercase())
            .any(|name| tag_names.contains(name.as_str()));

        if is_direct_child {
            return true;
        }

        for child in children {
            if Util::has_any_descendent_tag(&child, tag_names) {
                return true;
            }
        }

        false
    }

    pub fn has_ancestor_tag<F>(
        node: &Node,
        tag_name: &str,
        max_depth: Option<u64>,
        filter: Option<F>,
    ) -> bool
    where
        F: Fn(&Node) -> bool,
    {
        let max_depth = max_depth.unwrap_or(3);
        let tag_name = tag_name.to_uppercase();
        let mut depth = 0;
        let mut node = node.get_parent();

        loop {
            if depth > max_depth {
                return false;
            }

            let tmp_node = match node {
                Some(node) => node,
                None => return false,
            };

            if tmp_node.get_name().to_uppercase() == tag_name
                && filter
                    .as_ref()
                    .map(|filter| filter(&tmp_node))
                    .unwrap_or(true)
            {
                return true;
            }

            node = tmp_node.get_parent();
            depth += 1;
        }
    }

    pub fn has_single_tag_inside_element(node: &Node, tag: &str) -> bool {
        // There should be exactly 1 element child with given tag
        if node.get_child_elements().len() != 1
            || node
                .get_child_elements()
                .first()
                .map(|n| n.get_name().to_uppercase() != tag)
                .unwrap_or(false)
        {
            return false;
        }

        // And there should be no text nodes with real content
        !node.get_child_nodes().iter().any(|n| {
            n.get_type()
                .map(|t| t == NodeType::TextNode)
                .unwrap_or(false)
                && constants::HAS_CONTENT.is_match(&n.get_content())
        })
    }

    pub fn is_element_without_content(node: &Node) -> bool {
        if let Some(node_type) = node.get_type() {
            let len = node.get_child_nodes().len();

            node_type == NodeType::ElementNode
                && (len == 0
                    || len
                        == Self::get_elements_by_tag_name(node, "br").len()
                            + Self::get_elements_by_tag_name(node, "hr").len())
                && node.get_content().trim().is_empty()
        } else {
            false
        }
    }

    pub fn is_element_without_children(node: &Node) -> bool {
        if let Some(node_type) = node.get_type() {
            let len = node.get_child_nodes().len();
            node_type == NodeType::ElementNode
                && (len == 0 || node.get_content().trim().is_empty())
                && Self::get_elements_by_tag_names(node, &constants::VALID_EMPTY_TAGS).is_empty()
        } else {
            false
        }
    }

    pub fn get_elements_by_tag_names(node: &Node, tags: &HashSet<&str>) -> Vec<Node> {
        let mut vec = Vec::new();

        fn get_elems(node: &Node, tags: &HashSet<&str>, vec: &mut Vec<Node>) {
            for child in node.get_child_elements() {
                if tags.contains(child.get_name().to_uppercase().as_str()) {
                    vec.push(child.clone());
                }
                get_elems(&child, tags, vec);
            }
        }

        get_elems(node, tags, &mut vec);
        vec
    }

    pub fn get_elements_by_tag_name(node: &Node, tag: &str) -> Vec<Node> {
        let tag = tag.to_uppercase();
        let all_tags = tag == "*";
        let mut vec = Vec::new();

        fn get_elems(node: &Node, tag: &str, vec: &mut Vec<Node>, all_tags: bool) {
            for child in node.get_child_elements() {
                if all_tags || child.get_name().to_uppercase() == tag {
                    vec.push(child.clone());
                }
                get_elems(&child, tag, vec, all_tags);
            }
        }

        get_elems(node, &tag, &mut vec, all_tags);
        vec
    }

    pub fn get_first_element_by_tag_name(node: &Node, tag: &str) -> Option<Node> {
        let tag = tag.to_uppercase();

        fn get_elems(node: &Node, tag: &str) -> Option<Node> {
            for child in node.get_child_elements() {
                if child.get_name().to_uppercase() == tag {
                    return Some(child);
                } else if let Some(node) = get_elems(&child, tag) {
                    return Some(node);
                }
            }

            None
        }

        get_elems(node, &tag)
    }

    pub fn get_link_density(node: &Node) -> f64 {
        let text_length = Util::get_inner_text(node, true).len();
        if text_length == 0 {
            return 0.0;
        }

        let mut link_length = 0.0;

        // XXX implement _reduceNodeList?
        let link_nodes = Util::get_elements_by_tag_name(node, "A");
        for link_node in link_nodes {
            if let Some(href) = link_node.get_attribute("href") {
                let coefficient = if constants::HASH_URL.is_match(&href) {
                    0.3
                } else {
                    1.0
                };
                link_length += Util::get_inner_text(&link_node, true).len() as f64 * coefficient;
            }
        }

        link_length / text_length as f64
    }

    // Determine whether element has any children block level elements.
    pub fn has_child_block_element(node: &Node) -> bool {
        node.get_child_nodes().iter().any(|node| {
            constants::DIV_TO_P_ELEMS.contains(node.get_name().to_uppercase().as_str())
                || Self::has_child_block_element(node)
        })
    }

    pub fn get_node_ancestors(node: &Node, max_depth: Option<u64>) -> Vec<Node> {
        let mut ancestors = Vec::new();
        let mut node = node.clone();
        let max_depth = max_depth.unwrap_or(u64::MAX);

        for _ in 0..max_depth {
            let parent = node.get_parent();
            match parent {
                Some(parent) => {
                    ancestors.push(parent.clone());
                    node = parent;
                }
                None => return ancestors,
            }
        }

        ancestors
    }

    pub fn has_tag_name(node: Option<&Node>, tag_name: &str) -> bool {
        node.map(|n| n.get_name().to_uppercase() == tag_name.to_uppercase())
            .unwrap_or(false)
    }

    // Check if node is image, or if node contains exactly only one image
    // whether as a direct child or as its descendants.
    pub fn is_single_image(node: &Node) -> bool {
        if node.get_name().to_uppercase() == "IMG" {
            true
        } else if node.get_child_elements().len() != 1 || node.get_content().trim() != "" {
            false
        } else if let Some(first_child) = node.get_child_elements().first() {
            Self::is_single_image(first_child)
        } else {
            false
        }
    }

    pub fn clean_headers(root: &mut Node) {
        let mut nodes = Util::get_elements_by_tag_name(root, "h1");
        nodes.append(&mut Util::get_elements_by_tag_name(root, "h2"));

        for mut node in nodes.into_iter().rev() {
            if node.is_null() {
                continue;
            }

            if Util::get_class_weight(&node) < 0 {
                log::debug!(
                    "Removing header with low class weight: {} {}",
                    node.get_name(),
                    node.get_attribute("class").unwrap_or_default()
                );
                node.unlink();
            }
        }
    }

    pub fn replace_schema_org_orbjects(root: &mut Node) {
        let nodes = Util::get_elements_by_tag_name(root, "div");

        for mut node in nodes.into_iter().rev() {
            if let Some(video_object) = VideoObject::parse_node(&node) {
                _ = video_object.replace(&mut node);
            } else if let Some(image_object) = ImageObject::parse_node(&node) {
                _ = image_object.replace(&mut node);
            }
        }
    }

    pub fn replace_emoji_images(root: &Node, document: &Document) {
        let img_nodes = Util::get_elements_by_tag_name(root, "img");

        for img_node in img_nodes {
            if let Some(img_alt) = img_node.get_attribute("alt") {
                if Self::is_emoji(&img_alt) {
                    if let Some(mut parent) = img_node.get_parent() {
                        let emoji_text_node = Node::new_text(&img_alt, document).unwrap();
                        _ = parent.replace_child_node(emoji_text_node, img_node);
                    }
                }
            }
        }
    }

    pub fn is_emoji(text: &str) -> bool {
        let mut alt_chars = text.chars();
        let first_char = alt_chars.next();
        let second_char = alt_chars.next();

        if let (Some(char), None) = (first_char, second_char) {
            unic_emoji_char::is_emoji(char)
        } else {
            false
        }
    }

    // Clean an element of all tags of type "tag" if they look fishy.
    // "Fishy" is an algorithm based on content length, classnames, link density, number of images & embeds, etc.
    pub fn clean_conditionally(root: &mut Node, tag: &str) {
        // Gather counts for other typical elements embedded within.
        // Traverse backwards so we can remove nodes at the same time
        // without effecting the traversal.
        //
        // TODO: Consider taking into account original contentScore here.
        let nodes = Util::get_elements_by_tag_name(root, tag);

        for mut node in nodes.into_iter().rev() {
            if node.is_null() {
                continue;
            }

            if Self::should_remove(&node, tag) {
                node.unlink();
            }
        }
    }

    fn should_remove(node: &Node, tag: &str) -> bool {
        // First check if this node IS data table, in which case don't remove it.
        let mut is_list = tag == "ul" || tag == "ol";
        if !is_list {
            let mut list_length = 0.0;
            let ul_nodes = Self::get_elements_by_tag_name(node, "ul");
            let ol_nodes = Self::get_elements_by_tag_name(node, "ol");
            for list_node in ul_nodes {
                list_length += Util::get_inner_text(&list_node, false).len() as f64;
            }
            for list_node in ol_nodes {
                list_length += Util::get_inner_text(&list_node, false).len() as f64;
            }
            is_list = (list_length / Util::get_inner_text(node, false).len() as f64) > 0.9;
        }

        if tag == "table" && Self::is_data_table(node) {
            return false;
        }

        // Next check if we're inside a data table, in which case don't remove it as well.
        if Self::has_ancestor_tag(node, "table", Some(u64::MAX), Some(Self::is_data_table)) {
            return false;
        }

        if Self::has_ancestor_tag(node, "code", None, None::<fn(&Node) -> bool>) {
            return false;
        }

        let weight = Self::get_class_weight(node);
        if weight < 0 {
            return true;
        }

        if Self::get_char_count(node, ',') < 10 {
            // If there are not very many commas, and the number of
            // non-paragraph elements is more than paragraphs or other
            // ominous signs, remove the element.
            let p = Self::get_elements_by_tag_name(node, "p").len();
            let img = Self::get_elements_by_tag_name(node, "img").len();
            let li = Self::get_elements_by_tag_name(node, "li").len() as i64 - 100;
            let input = Self::get_elements_by_tag_name(node, "input").len();
            let heading_density =
                Self::get_text_density(node, &["h1", "h2", "h3", "h4", "h5", "h6"]);

            let mut embed_count = 0;
            let embed_tags = ["object", "embed", "iframe"];

            for embed_tag in embed_tags {
                for embed_node in Self::get_elements_by_tag_name(node, embed_tag) {
                    // If this embed has attribute that matches video regex, don't delete it.
                    for (_name, value) in embed_node.get_attributes() {
                        if constants::VIDEOS.is_match(&value) {
                            return false;
                        }
                    }

                    embed_count += 1;
                }
            }

            let link_density = Self::get_link_density(node);
            let content = Self::get_inner_text(node, true);
            let content_length = content.len();
            let has_figure_ancestor =
                Self::has_ancestor_tag(node, "figure", None, None::<fn(&Node) -> bool>);

            let image_obj_count = Util::get_elements_by_tag_name(node, "imageobject").len();
            let video_obj_count = Util::get_elements_by_tag_name(node, "videoobject").len();
            let video_tag_count = Util::get_elements_by_tag_name(node, "video").len();

            if image_obj_count > 0 || video_obj_count > 0 || video_tag_count > 0 {
                return false;
            }

            let have_to_remove = (img > 1 && (p as f64 / img as f64) < 0.5 && !has_figure_ancestor)
                || (!is_list && li > p as i64)
                || (input as f64 > f64::floor(p as f64 / 3.0))
                || (!is_list
                    && heading_density < 0.9
                    && content_length < 25
                    && (img == 0 || img > 2)
                    && !has_figure_ancestor)
                || (!is_list && weight < 25 && link_density > 0.2)
                || (weight >= 25 && link_density > 0.5)
                || ((embed_count == 1 && content_length < 75) || embed_count > 1);

            // Allow simple lists of images to remain in pages
            if is_list && have_to_remove {
                for child in node.get_child_elements() {
                    // Don't filter in lists with li's that contain more than one child
                    if child.get_child_elements().len() > 1 {
                        return have_to_remove;
                    }
                }

                let li_count = Util::get_elements_by_tag_name(node, "li").len();
                // Only allow the list to remain if every li contains an image
                if img == li_count {
                    return false;
                }
            }

            have_to_remove
        } else {
            false
        }
    }

    pub fn get_class_weight(node: &Node) -> i64 {
        let mut weight = 0;

        // Look for a special classname
        if let Some(class_names) = node.get_property("class") {
            if constants::NEGATIVE.is_match(&class_names) {
                weight -= 25;
            }

            if constants::POSITIVE.is_match(&class_names) {
                weight += 25;
            }
        }

        // Look for a special ID
        if let Some(class_names) = node.get_property("id") {
            if constants::NEGATIVE.is_match(&class_names) {
                weight -= 25;
            }

            if constants::POSITIVE.is_match(&class_names) {
                weight += 25;
            }
        }

        weight
    }

    fn get_char_count(node: &Node, char: char) -> usize {
        Util::get_inner_text(node, false).split(char).count() - 1
    }

    fn get_text_density(node: &Node, tags: &[&str]) -> f64 {
        let text_length = Util::get_inner_text(node, false).len();
        if text_length == 0 {
            return 0.0;
        }

        let mut children_length = 0;
        for tag in tags {
            for child in Self::get_elements_by_tag_name(node, tag) {
                children_length += Util::get_inner_text(&child, false).len()
            }
        }
        children_length as f64 / text_length as f64
    }

    fn is_data_table(node: &Node) -> bool {
        node.get_attribute(constants::DATA_TABLE_ATTR)
            .and_then(|is_data_table| is_data_table.parse::<bool>().ok())
            .unwrap_or(false)
    }

    pub fn mark_data_tables(context: &Context) -> Result<(), FullTextParserError> {
        let nodes = Util::evaluate_xpath(context, "//table", false)?;
        for mut node in nodes {
            if node
                .get_attribute("role")
                .map(|role| role == "presentation")
                .unwrap_or(false)
            {
                let _ = node.set_attribute(constants::DATA_TABLE_ATTR, "false");
                continue;
            }

            if node
                .get_attribute("datatable")
                .map(|role| role == "0")
                .unwrap_or(false)
            {
                let _ = node.set_attribute(constants::DATA_TABLE_ATTR, "false");
                continue;
            }

            if node.get_attribute("summary").is_some() {
                let _ = node.set_attribute(constants::DATA_TABLE_ATTR, "true");
                continue;
            }

            if let Some(first_caption) = Self::get_elements_by_tag_name(&node, "caption").first() {
                if !first_caption.get_child_nodes().is_empty() {
                    let _ = node.set_attribute(constants::DATA_TABLE_ATTR, "true");
                    continue;
                }
            }

            // If the table has a descendant with any of these tags, consider a data table:
            let data_table_descendants = ["col", "colgroup", "tfoot", "thead", "th"];
            for descendant in data_table_descendants {
                if !Self::get_elements_by_tag_name(&node, descendant).is_empty() {
                    let _ = node.set_attribute(constants::DATA_TABLE_ATTR, "true");
                    continue;
                }
            }

            // Nested tables indicate a layout table:
            if !Self::get_elements_by_tag_name(&node, "table").is_empty() {
                let _ = node.set_attribute(constants::DATA_TABLE_ATTR, "false");
                continue;
            }

            let (rows, columns) = Self::get_row_and_column_count(&node);
            if rows >= 10 || columns > 4 {
                let _ = node.set_attribute(constants::DATA_TABLE_ATTR, "true");
                continue;
            }

            // Now just go by size entirely:
            let _ = node.set_attribute(
                constants::DATA_TABLE_ATTR,
                if rows * columns > 10 { "true" } else { "false" },
            );
        }

        Ok(())
    }

    pub fn get_row_and_column_count(node: &Node) -> (usize, usize) {
        if node.get_name().to_uppercase() != "TABLE" {
            return (0, 0);
        }

        let mut rows = 0;
        let mut columns = 0;

        let trs = Self::get_elements_by_tag_name(node, "tr");
        for tr in trs {
            let row_span = tr
                .get_attribute("rowspan")
                .and_then(|span| span.parse::<usize>().ok())
                .unwrap_or(1);
            rows += row_span;

            // Now look for column-related info
            let mut columns_in_this_row = 0;
            let cells = Self::get_elements_by_tag_name(&tr, "td");
            for cell in cells {
                let colspan = cell
                    .get_attribute("colspan")
                    .and_then(|span| span.parse::<usize>().ok())
                    .unwrap_or(1);
                columns_in_this_row += colspan;
            }
            columns = usize::max(columns, columns_in_this_row);
        }

        (rows, columns)
    }

    pub fn is_phrasing_content(node: &Node) -> bool {
        let tag_name = node.get_name().to_uppercase();
        let is_text_node = node
            .get_type()
            .map(|t| t == NodeType::TextNode)
            .unwrap_or(false);

        is_text_node
            || constants::PHRASING_ELEMS.contains(&tag_name.as_str())
            || ((tag_name == "A" || tag_name == "DEL" || tag_name == "INS")
                && node.get_child_nodes().iter().all(Self::is_phrasing_content))
    }

    // Replaces 2 or more successive <br> elements with a single <p>.
    // Whitespace between <br> elements are ignored.
    // For example:
    //   <div>foo<br>bar<br> <br><br>abc</div>
    // will become:
    //   <div>foo<br>bar<p>abc</p></div>
    pub fn replace_brs(node: &Node, document: &Document) {
        let br_nodes = Self::get_elements_by_tag_name(node, "br");

        for br_node in br_nodes {
            let mut next = br_node.get_next_sibling();

            // Whether 2 or more <br> elements have been found and replaced with a
            // <p> block.
            let mut replaced = false;

            // If we find a <br> chain, remove the <br>s until we hit another node
            // or non-whitespace. This leaves behind the first <br> in the chain
            // (which will be replaced with a <p> later).
            while let Some(mut n) = next {
                if n.is_null() {
                    break;
                }

                let is_text_whitespace = n
                    .get_type()
                    .map(|t| t == NodeType::TextNode)
                    .unwrap_or(false)
                    && n.get_content().trim().is_empty();
                let is_br_node = n.get_name().to_uppercase() == "BR";
                let next_is_br_node = n
                    .get_next_sibling()
                    .map(|n| n.get_name().to_uppercase() == "BR")
                    .unwrap_or(false);

                if !is_text_whitespace && !is_br_node {
                    break;
                }

                next = n.get_next_sibling();

                if is_br_node || (is_text_whitespace && next_is_br_node) {
                    replaced = true;
                    n.unlink();
                }
            }

            if !replaced {
                continue;
            }

            // If we removed a <br> chain, replace the remaining <br> with a <p>. Add
            // all sibling nodes as children of the <p> until we hit another <br>
            // chain.
            let mut parent = match br_node.get_parent() {
                Some(parent) => parent,
                None => continue,
            };
            let mut p = Node::new("p", None, document).unwrap();
            _ = parent.replace_child_node(p.clone(), br_node).unwrap();

            next = p.get_next_sibling();

            while let Some(mut next_node) = next {
                if next_node.is_null() {
                    break;
                }

                // If we've hit another <br><br>, we're done adding children to this <p>.
                if next_node.get_name().to_uppercase() == "BR" {
                    if let Some(next_elem) = next_node.get_next_element_sibling() {
                        if next_elem.get_name().to_uppercase() == "BR" {
                            break;
                        }
                    }
                }

                if !Self::is_phrasing_content(&next_node) {
                    break;
                }

                // Otherwise, make this node a child of the new <p>.
                let sibling = next_node.get_next_sibling();
                next_node.unlink();
                _ = p.add_child(&mut next_node);

                next = sibling;
            }

            if p.get_child_elements().is_empty() && p.get_content().trim().is_empty() {
                p.unlink();
                continue;
            }

            while let Some(mut last_child) = p.get_last_child() {
                if last_child.is_null() {
                    continue;
                }

                let is_text_node = last_child
                    .get_type()
                    .map(|t| t == NodeType::TextNode)
                    .unwrap_or(false);
                let is_empty = last_child.get_content().trim().is_empty();

                if is_text_node && is_empty {
                    last_child.unlink();
                } else {
                    break;
                }
            }

            if let Some(mut parent) = p.get_parent() {
                if parent.get_name().to_uppercase() == "P" {
                    _ = parent.set_name("DIV");
                }
            }
        }
    }

    pub fn score_image_url(url: &str) -> i32 {
        let url = url.trim();
        let mut score = 0;

        if constants::POSITIVE_LEAD_IMAGE_URL_HINTS_REGEX.is_match(url) {
            score += 20;
        }

        if NEGATIVE_LEAD_IMAGE_URL_HINTS_REGEX.is_match(url) {
            score -= 20;
        }

        // TODO: We might want to consider removing this as
        // gifs are much more common/popular than they once were
        if constants::GIF_REGEX.is_match(url) {
            score -= 10;
        }

        if constants::JPG_REGEX.is_match(url) {
            score += 10;
        }

        // PNGs are neutral.

        score
    }

    // Alt attribute usually means non-presentational image.
    pub fn score_img_attr(img: &Node) -> i32 {
        if img.get_attribute("alt").is_some() {
            5
        } else {
            0
        }
    }

    // Look through our parent and grandparent for figure-like
    // container elements, give a bonus if we find them
    pub fn score_by_parents(img: &Node) -> i32 {
        let mut score = 0;
        let parent = img.get_parent();
        let grand_parent = parent.as_ref().and_then(|n| n.get_parent());
        if Self::has_tag_name(parent.as_ref(), "figure")
            || Self::has_tag_name(grand_parent.as_ref(), "figure")
        {
            score += 25;
        }

        if let Some(parent) = parent.as_ref() {
            let signature = Util::get_signature(parent);
            if constants::PHOTO_HINTS_REGEX.is_match(&signature) {
                score += 15;
            }
        }

        if let Some(grand_parent) = grand_parent.as_ref() {
            let signature = Util::get_signature(grand_parent);
            if constants::PHOTO_HINTS_REGEX.is_match(&signature) {
                score += 15;
            }
        }

        score
    }

    // Look at our immediate sibling and see if it looks like it's a
    // caption. Bonus if so.
    pub fn score_by_sibling(img: &Node) -> i32 {
        let mut score = 0;
        let sibling = img.get_next_element_sibling();

        if let Some(sibling) = sibling.as_ref() {
            if sibling.get_name().to_lowercase() == "figcaption" {
                score += 25;
            }

            let signature = Util::get_signature(sibling);
            if constants::PHOTO_HINTS_REGEX.is_match(&signature) {
                score += 15;
            }
        }

        score
    }

    pub fn score_by_dimensions(img: &Node) -> i32 {
        let mut score = 0;

        let width = img
            .get_attribute("width")
            .and_then(|w| w.parse::<f32>().ok());
        let height = img
            .get_attribute("height")
            .and_then(|w| w.parse::<f32>().ok());
        let src = img.get_attribute("src").unwrap_or_default();

        // Penalty for skinny images
        if let Some(width) = width {
            if width <= 50.0 {
                score -= 50;
            }
        }

        // Penalty for short images
        if let Some(height) = height {
            if height <= 50.0 {
                score -= 50;
            }
        }

        if let (Some(width), Some(height)) = (width, height) {
            if !src.contains("sprite") {
                let area = width * height;
                if area < 5000.0 {
                    // Smaller than 50 x 100
                    score -= 100;
                } else {
                    score += f32::round(area / 1000.0) as i32;
                }
            }
        }

        score
    }

    pub fn score_by_position(len: usize, index: usize) -> i32 {
        ((len as f32 / 2.0) - index as f32) as i32
    }

    pub fn score_by_alt(node: &Node) -> i32 {
        if let Some(alt) = node.get_attribute("alt") {
            if Self::is_emoji(&alt) {
                -100
            } else {
                0
            }
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Util;
    use crate::FullTextParser;

    fn replace_brs(source: &str, expected: &str) {
        libxml::tree::node::set_node_rc_guard(10);

        let document = FullTextParser::parse_html_string_patched(source).unwrap();
        let root = document.get_root_element().unwrap();
        let body = root.get_first_child().unwrap();
        let div = body.get_first_child().unwrap();

        Util::replace_brs(&root, &document);

        let result = document.node_to_string(&div);

        assert_eq!(expected, result);
    }

    #[test]
    fn replace_brs_1() {
        replace_brs(
            "<div>foo<br>bar<br> <br><br>abc</div>",
            "<div>foo<br/>bar<p>abc</p></div>",
        )
    }

    #[test]
    fn replace_brs_2() {
        let source = r#"
        <div>
            <p>
                It might have been curiosity or it might have been the nagging sensation that chewed at his brain for the three weeks that he researched the subject of the conversation. All For One was a cryptid. Mystical in more ways than one, he was only a rumour on a network that was two-hundred years old. There were whispers of a shadowy figure who once ruled Japan, intermingled with a string of conspiracies and fragmented events.
            </p>
            <p>
                Izuku had even braved the dark web, poking and prodding at some of the seedier elements of the world wide web. The internet had rumours, but the dark web had stories.<br/>
            </p>
            <p>
                An implied yakuza wrote about his grandfather who lost a fire manipulation Quirk and his sanity without any reason. His grandfather had been institutionalised, crying and repeating “he took it, he took it” until his dying days. No one could console him.
            </p>
        </div>
        "#;
        replace_brs(source, source.trim())
    }

    fn replace_emojis(source: &str, expected: &str) {
        libxml::tree::node::set_node_rc_guard(10);

        let document = FullTextParser::parse_html_string_patched(source).unwrap();
        let root = document.get_root_element().unwrap();
        let body = root.get_first_child().unwrap();
        let p = body.get_first_child().unwrap();

        Util::replace_emoji_images(&root, &document);

        let result = document.node_to_string(&p);

        assert_eq!(expected, result);
    }

    #[test]
    fn replace_emojis_1() {
        replace_emojis(
            "<p>Let’s see if I did a better job of it this time by telling him he was using Arch wrong. <img src=\"https://s0.wp.com/wp-content/mu-plugins/wpcom-smileys/twemoji/2/72x72/1f600.png\" alt=\"😀\"/></p>",
            "<p>Let’s see if I did a better job of it this time by telling him he was using Arch wrong. 😀</p>",
        )
    }

    #[test]
    fn replace_emojis_2() {
        replace_emojis(
            "<p><img src=\"https://abc.com/img.jpeg\"/><img src=\"https://s0.wp.com/wp-content/mu-plugins/wpcom-smileys/twemoji/2/72x72/1f600.png\" alt=\"😀\"/> Abc</p>",
            "<p><img src=\"https://abc.com/img.jpeg\"/>😀 Abc</p>",
        )
    }
}
