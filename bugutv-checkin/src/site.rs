use crate::config::AppConfig;

use anyhow::{Error, Ok, Result};
use regex::Regex;
use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, ORIGIN, REFERER},
};
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
        let success = self.login().await?;
        if !success {
            return Err(Error::msg("登陆失败"));
        }
        let nonce = self.get_nonce().await?;
        info!("获取nonce: {}", nonce);
        self.check_in(nonce).await?;
        Ok(())
    }

    pub async fn login(&self) -> Result<bool> {
        let _ = self.client.get(self.cfg.base_url.clone()).send().await?;

        let url = self.cfg.base_url.clone() + "/wp-login.php";

        let mut form = HashMap::new();

        form.insert("log", self.cfg.username.clone());
        form.insert("pwd", self.cfg.password.clone());
        form.insert("wp-submit", "登录".into());
        form.insert("redirect_to", self.cfg.base_url.clone());
        form.insert("testcookie", "1".into());

        let mut headers = HeaderMap::new();
        headers.insert(REFERER, url.parse().unwrap());
        headers.insert(ORIGIN, self.cfg.base_url.clone().parse().unwrap());

        let resp = self
            .client
            .post(url)
            .headers(headers)
            .form(&form)
            .send()
            .await?;
        match resp.status() {
            StatusCode::OK => {
                let re = Regex::new(r"(?ms)(积分钱包).*(当前余额：)(\d+?)")?;
                let resp_text = resp.text().await?;
                if let Some(cap) = re.captures(&resp_text) {
                    if cap.len() == 4 {
                        info!("当前积分余额: {:?}", &cap[3]);
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
            StatusCode::FOUND => Ok(true),
            other => {
                warn!("login failed: {:?}", other);
                warn!("headers: {:?}", resp.headers());

                Ok(false)
            }
        }
    }

    pub async fn get_nonce(&self) -> Result<String> {
        let url_user = self.cfg.base_url.clone() + "/user";
        let nonce_re = Regex::new(r#"data-nonce="(.*?)""#).unwrap();

        let user_text = self.client.get(url_user).send().await?.text().await?;
        let Some(cap) = nonce_re.captures(&user_text) else {
            info!("没有找到nonce");
            return Err(Error::msg("not found nonce"));
        };
        let nonce = &cap[1];
        Ok(nonce.to_string())
    }

    pub async fn check_in(&self, nonce: String) -> Result<(), Error> {
        let url_checkin = self.cfg.base_url.clone() + "/wp-admin/admin-ajax.php";
        let response = self
            .client
            .post(url_checkin)
            .form(&json!({"action": "user_qiandao", "nonce": nonce}))
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

    fn setup_mockserver(path: &str, status: u16, body: impl AsRef<[u8]>) -> MockServer {
        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(POST).path(path);
            then.status(status).body(body);
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
        let client = client::from_url_with_default().unwrap();
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
        let client = client::from_url_with_default().unwrap();
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
            let client = client::from_url_with_default().unwrap();
            let site = BrowserSite::new(cfg, client);
            let resp = site.login().await;
            assert!(resp.is_ok());
            assert_eq!(resp.unwrap(), false);
        }
    }

    #[tokio::test]
    async fn test_site_login_with_content() {
        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(POST).path("/wp-login.php");
            then.status(200).body(
                r#"
            <div class="menu-card-box-1">
            <span class="small"><i class="fas fa-coins mr-1"></i>积分钱包</span>
            <p class="small m-0">当前余额：1</p><p class="small">累计消费：3</p>
            <a class="btn btn-sm btn-block btn-rounded btn-light" href="https://www.bugutv.vip/user/vip" rel="nofollow noopener noreferrer">我的会员</a></div>
            "#,
            );
        });

        let cfg = AppConfig {
            base_url: server.base_url(),
            username: "user1".into(),
            password: "passwd1".into(),
        };
        let client = client::from_url_with_default().unwrap();
        let site = BrowserSite::new(cfg, client);
        let resp = site.login().await;
        assert!(resp.is_ok());
        assert_eq!(resp.unwrap(), true);
    }

    #[tokio::test]
    async fn test_site_check_in() {
        struct Validate {
            result: bool,
            body: String,
        }
        let mock_bodys = vec![
            Validate {
                result: false,
                body: json!({"status": 0, "msg": "签到失败"}).to_string(),
            },
            Validate {
                result: false,
                body: json!({"status": 1, "msg": "签到成功"}).to_string(),
            },
            Validate {
                result: false,
                body: json!("<html></html>").to_string(),
            },
        ];

        for mock_body in mock_bodys {
            let server = setup_mockserver("/wp-admin/admin-ajax.php", 200, mock_body.body);
            let cfg = AppConfig {
                base_url: server.base_url(),
                username: "user1".into(),
                password: "passwd1".into(),
            };
            let client = client::from_url_with_default().unwrap();
            let site = BrowserSite::new(cfg, client);
            let resp = site.check_in("xxxx".into()).await;
            if mock_body.result {
                assert!(resp.is_ok());
            } else {
                assert!(resp.is_err())
            }
        }
    }

    #[tokio::test]
    async fn test_site_nonce() {
        struct MockData {
            result: Option<String>,
            body: String,
            status: u16,
        }

        let mock_datas = vec![
            MockData {
                result: Some("28b21150cd".into()),
                body: r#"sdjis data-nonce="28b21150cd" data-toggle="tooltip">"#.into(),
                status: 200,
            },
            MockData {
                result: None,
                body: "<html></html>".into(),
                status: 200,
            },
            MockData {
                result: None,
                body: "".into(),
                status: 503,
            },
        ];

        for mock in mock_datas {
            let server = MockServer::start();

            server.mock(|when, then| {
                when.method(GET).path("/user");
                then.status(mock.status).body(mock.body);
            });

            let cfg = AppConfig {
                base_url: server.base_url(),
                username: "user1".into(),
                password: "passwd1".into(),
            };
            let client = client::from_url_with_default().unwrap();
            let site = BrowserSite::new(cfg, client);
            let nonce = site.get_nonce().await.ok();

            assert_eq!(nonce, mock.result)
        }
    }
}
