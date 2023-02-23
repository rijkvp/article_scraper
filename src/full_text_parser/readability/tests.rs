use libxml::{
    tree::{Document, Node},
    xpath::Context,
};
use reqwest::Url;

use crate::{
    article::Article,
    full_text_parser::{config::ConfigEntry, metadata},
};

async fn prepare(html: &str, url: &Url) -> (Document, Context, Article) {
    let empty_config = ConfigEntry::default();
    let document = crate::FullTextParser::parse_html(html, None, &empty_config).unwrap();
    let xpath_ctx = crate::FullTextParser::get_xpath_ctx(&document).unwrap();
    crate::FullTextParser::strip_junk(&xpath_ctx, None, &empty_config, url);
    crate::FullTextParser::unwrap_noscript_images(&xpath_ctx).unwrap();
    let article = Article {
        title: None,
        author: None,
        url: url.clone(),
        date: None,
        thumbnail_url: None,
        document: None,
    };
    (document, xpath_ctx, article)
}

#[tokio::test]
async fn test_1() {
    let _ = env_logger::builder().is_test(true).try_init();

    let html = std::fs::read_to_string(r"./resources/tests/readability-test-1.html")
        .expect("Failed to read HTML");
    let url = Url::parse("http://google.com").unwrap();
    let (document, xpath_ctx, mut article) = prepare(&html, &url).await;

    let mut article_document = Document::new().unwrap();
    let mut root = Node::new("article", None, &document).unwrap();
    article_document.set_root_element(&root);

    metadata::extract(&xpath_ctx, None, None, &mut article);
    super::Readability::extract_body(document, &mut root, article.title.as_deref()).unwrap();

    if let Some(mut root) = article_document.get_root_element() {
        crate::FullTextParser::post_process_content(&mut root).unwrap();
    }

    article.document = Some(article_document);
    let html = article.get_content().unwrap();
    std::fs::write("test.html", html).unwrap();
}
