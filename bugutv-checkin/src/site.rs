use crate::config::AppConfig;

use anyhow::{Error, Result};
use regex::Regex;
use reqwest::{Client, StatusCode};
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

    pub async fn login(&self) -> Result<bool> {
        let url = self.cfg.base_url.clone() + "/wp-login.php";

        let mut form = HashMap::new();

        form.insert("log", self.cfg.username.clone());
        form.insert("pwd", self.cfg.password.clone());
        form.insert("wp-submit", "登录".into());
        form.insert("redirect_to", self.cfg.base_url.clone());
        form.insert("testcookie", "1".into());

        let resp = self.client.post(url).form(&form).send().await?;
        match resp.status() {
            StatusCode::FOUND => Ok(true),
            _ => Ok(false),
        }
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

#[cfg(test)]
mod tests {
    use crate::client;

    use super::*;
    use httpmock::prelude::*;

    fn mockserver_by_status(status: u16) -> MockServer {
        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(POST).path("/wp-login.php");
            then.status(status)
                .header(
                    "set-cookie",
                    "wordpress_test_cookie=WP-Cookie-Check; path=/; secure",
                )
                .body("<html></html>");
        });
        server
    }

    #[tokio::test]
    async fn test_site_login_status200() {
        let server = mockserver_by_status(200);

        // let url = format!("{}/wp-login.php", server.base_url());
        let cfg = AppConfig {
            base_url: server.base_url(),
            username: "user1".into(),
            password: "passwd1".into(),
        };
        let client = client::from_url_with_default(server.base_url()).unwrap();
        let site = BrowserSite::new(cfg, client);
        let resp = site.login().await;
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap(), false);
    }

    #[tokio::test]
    async fn test_site_login_status302() {
        let server = mockserver_by_status(302);

        // let url = format!("{}/wp-login.php", server.base_url());
        let cfg = AppConfig {
            base_url: server.base_url(),
            username: "user1".into(),
            password: "passwd1".into(),
        };
        let client = client::from_url_with_default(server.base_url()).unwrap();
        let site = BrowserSite::new(cfg, client);
        let resp = site.login().await;
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap(), true);
    }

    #[tokio::test]
    async fn test_site_login_some_status() {
        let statuses: [u16; 9] = [200, 201, 401, 403, 500, 501, 502, 503, 504];

        for status in statuses {
            let server = mockserver_by_status(status);
            let cfg = AppConfig {
                base_url: server.base_url(),
                username: "user1".into(),
                password: "passwd1".into(),
            };
            let client = client::from_url_with_default(server.base_url()).unwrap();
            let site = BrowserSite::new(cfg, client);
            let resp = site.login().await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap(), false);
        }
    }
}
