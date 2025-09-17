use anyhow::Result;
use reqwest::{
    Client, ClientBuilder,
    header::{CONTENT_TYPE, HeaderMap},
};

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36";

pub fn from_url_with_default() -> Result<Client> {
    // let redirect_policy = Policy::none();
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        "application/x-www-form-urlencoded".parse().unwrap(),
    );

    let client = ClientBuilder::new()
        .user_agent(USER_AGENT)
        // .redirect(redirect_policy)
        .cookie_store(true)
        .default_headers(headers)
        .build()?;
    Ok(client)
}
