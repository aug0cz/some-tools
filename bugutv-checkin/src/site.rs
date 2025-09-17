use crate::config::AppConfig;

use anyhow::{Error, Result};
use regex::Regex;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Debug)]
pub struct BrowserSite {
    cfg: AppConfig,
    client: Client,
}

impl BrowserSite {
    pub fn new(cfg: AppConfig, client: Client) -> Self {
        Self {
            cfg: cfg,
            client: client,
        }
    }

    pub async fn login_and_check_in(&self) -> Result<()> {
        self.login().await?;
        self.check_in().await?;
        Ok(())
    }

    pub async fn login(&self) -> Result<()> {
        let url = self.cfg.base_url.clone() + "/wp-login.php";

        let mut form = HashMap::new();

        form.insert("log", self.cfg.username.clone());
        form.insert("pwd", self.cfg.password.clone());
        form.insert("wp-submit", "登录".into());
        form.insert("redirect_to", self.cfg.base_url.clone());
        form.insert("testcookie", "1".into());
        let _ = self.client.post(url).form(&form).send().await?;
        Ok(())
    }

    pub async fn check_in(&self) -> Result<(), Error> {
        let url_checkin = self.cfg.base_url.clone() + "/wp-admin/admin-ajax.php";
        let url_user = self.cfg.base_url.clone() + "/user";
        let nonce_re = Regex::new(r#"data-nonce="(.*?)""#).unwrap();

        let user_text = self.client.get(url_user).send().await?.text().await?;

        let Some(nonce) = nonce_re.captures(&user_text) else {
            info!("没有找到nonce");
            return Err(Error::msg("not found nonce"));
        };

        info!("find nonce: {:?}", &nonce[1]);

        let response = self
            .client
            .post(url_checkin)
            .form(&json!({"action": "user_qiandao", "nonce": &nonce[1]}))
            .send()
            .await?;

        let response_text = response.text().await?;
        let v = serde_json::from_str::<Value>(&response_text)?;

        if let Some(status) = v.get("status") {
            if *status == json!("1") {
                info!("签到成功");
                return Ok(());
            }
        }

        warn!("签到失败");
        Err(Error::msg("签到失败"))
    }
}
