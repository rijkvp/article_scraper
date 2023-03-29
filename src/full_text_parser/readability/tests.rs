use libxml::tree::{Document, Node};
use reqwest::Url;

use crate::{
    article::Article,
    full_text_parser::{config::ConfigEntry, metadata},
};

async fn run_test(name: &str) {
    libxml::tree::node::set_node_rc_guard(10);
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    let empty_config = ConfigEntry::default();

    let url = Url::parse("http://fakehost/test/base/").unwrap();
    let html = std::fs::read_to_string(format!("./resources/tests/readability/{name}/source.html"))
        .expect("Failed to read source HTML");
    let document = crate::FullTextParser::parse_html(&html, None, &empty_config).unwrap();
    let xpath_ctx = crate::FullTextParser::get_xpath_ctx(&document).unwrap();

    crate::FullTextParser::prep_content(&xpath_ctx, None, &empty_config, &url, &document);
    let mut article = Article {
        title: None,
        author: None,
        url,
        date: None,
        thumbnail_url: None,
        document: None,
    };

    let mut article_document = Document::new().unwrap();
    let mut root = Node::new("article", None, &document).unwrap();
    article_document.set_root_element(&root);

    metadata::extract(&xpath_ctx, None, None, &mut article);
    super::Readability::extract_body(document, &mut root, article.title.as_deref()).unwrap();
    crate::FullTextParser::post_process_document(&article_document).unwrap();

    article.document = Some(article_document);
    let html = article.get_content().unwrap();

    std::fs::write("expected.html", &html).unwrap();

    let expected = std::fs::read_to_string(format!(
        "./resources/tests/readability/{name}/expected.html"
    ))
    .expect("Failed to read expected HTML");

    assert_eq!(expected, html);
}

#[tokio::test]
async fn test_001() {
    run_test("001").await
}

#[tokio::test]
async fn test_002() {
    run_test("002").await
}

#[tokio::test]
async fn test_003() {
    run_test("003").await
}

#[tokio::test]
async fn aclu() {
    run_test("aclu").await
}

#[tokio::test]
async fn aktualne() {
    run_test("aktualne").await
}

#[tokio::test]
async fn archive_of_our_own() {
    run_test("archive-of-our-own").await
}

#[tokio::test]
async fn ars_1() {
    run_test("ars-1").await
}

#[tokio::test]
async fn base_url_base_element_relative() {
    run_test("base-url-base-element-relative").await
}

#[tokio::test]
async fn basic_tags_cleaning() {
    run_test("basic-tags-cleaning").await
}

#[tokio::test]
async fn bbc_1() {
    run_test("bbc-1").await
}

#[tokio::test]
async fn blogger() {
    run_test("blogger").await
}

#[tokio::test]
async fn breitbart() {
    run_test("breitbart").await
}

#[tokio::test]
async fn bug_1255978() {
    run_test("bug-1255978").await
}

#[tokio::test]
async fn buzzfeed_1() {
    run_test("buzzfeed-1").await
}

#[tokio::test]
async fn citylab_1() {
    run_test("citylab-1").await
}

#[tokio::test]
async fn clean_links() {
    run_test("clean-links").await
}

#[tokio::test]
async fn cnet_svg_classes() {
    run_test("cnet-svg-classes").await
}

#[tokio::test]
async fn cnet() {
    run_test("cnet").await
}

#[tokio::test]
async fn cnn() {
    run_test("cnn").await
}

#[tokio::test]
async fn comment_inside_script_parsing() {
    run_test("comment-inside-script-parsing").await
}

#[tokio::test]
async fn daringfireball_1() {
    run_test("daringfireball-1").await
}

#[tokio::test]
async fn data_url_image() {
    run_test("data-url-image").await
}

#[tokio::test]
async fn dev418() {
    run_test("dev418").await
}

#[tokio::test]
async fn dropbox_blog() {
    run_test("dropbox-blog").await
}

#[tokio::test]
async fn ebb_org() {
    run_test("ebb-org").await
}

#[tokio::test]
async fn ehow_1() {
    run_test("ehow-1").await
}

#[tokio::test]
async fn ehow_2() {
    run_test("ehow-2").await
}

#[tokio::test]
async fn embedded_videos() {
    run_test("embedded-videos").await
}

#[tokio::test]
async fn engadget() {
    run_test("engadget").await
}

#[tokio::test]
async fn firefox_nightly_blog() {
    run_test("firefox-nightly-blog").await
}

#[tokio::test]
async fn folha() {
    run_test("folha").await
}

#[tokio::test]
async fn gmw() {
    run_test("gmw").await
}

#[tokio::test]
async fn google_sre_book_1() {
    run_test("google-sre-book-1").await
}

#[tokio::test]
async fn guardian_1() {
    run_test("guardian-1").await
}

#[tokio::test]
async fn heise() {
    run_test("heise").await
}

#[tokio::test]
async fn herald_sun_1() {
    run_test("herald-sun-1").await
}

#[tokio::test]
async fn hidden_nodes() {
    run_test("hidden-nodes").await
}

#[tokio::test]
async fn hukumusume() {
    run_test("hukumusume").await
}

#[tokio::test]
async fn iab_1() {
    run_test("iab-1").await
}

#[tokio::test]
async fn ietf_1() {
    run_test("ietf-1").await
}

#[tokio::test]
async fn js_link_replacement() {
    run_test("js-link-replacement").await
}

#[tokio::test]
async fn keep_images() {
    run_test("keep-images").await
}

#[tokio::test]
async fn keep_tabular_data() {
    run_test("keep-tabular-data").await
}

#[tokio::test]
async fn la_nacion() {
    run_test("la-nacion").await
}

#[tokio::test]
async fn lazy_image_1() {
    run_test("lazy-image-1").await
}

#[tokio::test]
async fn lazy_image_2() {
    run_test("lazy-image-2").await
}

#[tokio::test]
async fn lazy_image_3() {
    run_test("lazy-image-3").await
}

#[tokio::test]
async fn lemonde_1() {
    run_test("lemonde-1").await
}

#[tokio::test]
async fn liberation_1() {
    run_test("liberation-1").await
}

#[tokio::test]
async fn lifehacker_post_comment_load() {
    run_test("lifehacker-post-comment-load").await
}

#[tokio::test]
async fn lifehacker_working() {
    run_test("lifehacker-working").await
}

#[tokio::test]
async fn links_in_tables() {
    run_test("links-in-tables").await
}

#[tokio::test]
async fn lwn_1() {
    run_test("lwn-1").await
}

// #[tokio::test]
// async fn medicalnewstoday() {
//     run_test("medicalnewstoday").await
// }

#[tokio::test]
async fn medium_1() {
    run_test("medium-1").await
}

#[tokio::test]
async fn medium_2() {
    run_test("medium-2").await
}

#[tokio::test]
async fn medium_3() {
    run_test("medium-3").await
}

#[tokio::test]
async fn webmd_1() {
    run_test("webmd-1").await
}
