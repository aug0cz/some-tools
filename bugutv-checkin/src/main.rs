use crate::{config::AppConfig, site::BrowserSite};
use anyhow::Result;
use tracing::info;

mod client;
mod config;
mod site;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let config = AppConfig::from_env()?;
    let client = client::from_url_with_default()?;
    let bugutv_site = BrowserSite::new(config, client);
    let status = bugutv_site.login().await?;
    info!("登陆状态: {}", status);

    let _ = bugutv_site.check_in().await?;

    Ok(())
}
