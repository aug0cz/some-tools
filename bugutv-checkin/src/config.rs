use anyhow::Result;
use dotenvy::dotenv;
use serde::Deserialize;
use std::env;

const BUGUTV_BASEURL: &str = "https://www.bugutv.vip";

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub username: String,
    pub password: String,
    pub base_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenv().ok();

        Ok(Self {
            username: env::var("USERNAME")?,
            password: env::var("PASSWORD")?,
            base_url: env::var("BASE_URL").unwrap_or(BUGUTV_BASEURL.into()),
        })
    }
}
