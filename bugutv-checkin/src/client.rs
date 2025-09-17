use anyhow::Result;
use reqwest::{Client, ClientBuilder};

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36";

pub fn from_url_with_default(base_url: String) -> Result<Client> {
    let client = ClientBuilder::new()
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .build()?;
    Ok(client)
}
