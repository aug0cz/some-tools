use crate::{config::AppConfig, site::BrowserSite};
use anyhow::Result;

mod client;
mod config;
mod site;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::from_env()?;
    let client = client::from_url_with_default(config.base_url.clone())?;
    let bugutv_site = BrowserSite::new(config, client);
    let _ = bugutv_site.login().await?;

    Ok(())
}
