use anyhow::{Context, Result, bail};
use reqwest::{Method, StatusCode, blocking::Client, header};
use serde_json::Value;
use std::{io::Read, time::Duration};
use url::Url;

pub const JSON_LIMIT: u64 = 64 * 1024 * 1024;
pub const DISCOVERY_LIMIT: u64 = 16 * 1024 * 1024;

pub(crate) fn live_session_only() -> bool {
    std::env::var_os("JOOM_LIVE_SESSION_ONLY").is_some_and(|value| value == "1")
}

/// Apply the same Microsoft Graph origin policy to contracts and requests.
pub fn contract_url(url: &Url) -> Result<()> {
    crate::microsoft::graph_url(url)
}

pub fn limited(mut reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.by_ref().take(limit + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= limit,
        "response exceeds {limit} byte limit"
    );
    Ok(bytes)
}

pub struct Http {
    pub client: Client,
}

impl Http {
    pub fn new(timeout: u64) -> Result<Self> {
        anyhow::ensure!(
            (1..=600).contains(&timeout),
            "--timeout must be between 1 and 600 seconds"
        );
        Ok(Self {
            client: Client::builder()
                .user_agent(concat!("joom/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(timeout))
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                // Avoid environment-controlled proxies receiving bearer credentials.
                .no_proxy()
                .gzip(true)
                .build()?,
        })
    }

    pub fn json(
        &self,
        method: &Method,
        url: &Url,
        body: Option<&Value>,
        token: Option<&str>,
        limit: u64,
    ) -> Result<Value> {
        self.json_conditional(method, url, body, token, limit, None)
    }

    pub fn json_conditional(
        &self,
        method: &Method,
        url: &Url,
        body: Option<&Value>,
        token: Option<&str>,
        limit: u64,
        etag: Option<&str>,
    ) -> Result<Value> {
        if token.is_some() {
            crate::microsoft::graph_url(url)?;
        }
        for attempt in 0..3 {
            let mut req = self.client.request(method.clone(), url.clone());
            if let Some(token) = token {
                let mut value = header::HeaderValue::from_str(&format!("Bearer {token}"))
                    .context("invalid access token")?;
                value.set_sensitive(true);
                req = req.header(header::AUTHORIZATION, value);
            }
            if let Some(etag) = etag {
                req = req.header(header::IF_MATCH, etag);
            }
            if let Some(body) = body {
                req = req.json(body);
            } else if matches!(*method, Method::POST | Method::PUT | Method::PATCH) {
                req = req.header(header::CONTENT_LENGTH, 0);
            }
            let response = req.send().map_err(|e| e.without_url())?;
            let status = response.status();
            let retry = method == Method::GET
                && !live_session_only()
                && matches!(status.as_u16(), 429 | 502 | 503 | 504)
                && attempt < 2;
            if retry {
                let wait = response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|h| h.parse::<u64>().ok());
                let mut random = [0_u8; 1];
                getrandom::fill(&mut random).context("operating system randomness unavailable")?;
                let delay = Duration::from_secs(wait.unwrap_or(1 << attempt).min(10))
                    + Duration::from_millis(u64::from(random[0]));
                std::thread::sleep(delay);
                continue;
            }
            // Do not echo arbitrary remote error bodies, which can include secrets.
            let response = check_response(response, method, url)?;
            let bytes = limited(response, limit)?;
            return if bytes.is_empty() {
                Ok(Value::Null)
            } else {
                serde_json::from_slice(&bytes)
                    .context("Microsoft returned invalid JSON; use --output for media")
            };
        }
        unreachable!("bounded retry loop always returns")
    }
}

pub fn check_response(
    response: reqwest::blocking::Response,
    _method: &Method,
    _url: &Url,
) -> Result<reqwest::blocking::Response> {
    #[cfg(feature = "live-test-diagnostics")]
    if !response.status().is_success() {
        let status = response.status();
        if let Ok(path) = std::env::var("JOOM_LIVE_ERROR_REPORT") {
            use std::io::Write;
            let code = limited(response, 16 * 1024)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .and_then(|value| {
                    value
                        .pointer("/error/code")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .filter(|code| {
                    code.len() <= 100
                        && code
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
                });
            let mut options = std::fs::OpenOptions::new();
            options.create(true).append(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            if let Ok(mut file) = options.open(path) {
                let _ = serde_json::to_writer(
                    &mut file,
                    &serde_json::json!({"status":status.as_u16(),"code":code,"method":_method.as_str(),"path":_url.path()}),
                );
                let _ = writeln!(file);
            }
            check_status(status)?;
            unreachable!("non-successful status is an error");
        }
    }
    check_status(response.status())?;
    Ok(response)
}

pub fn check_status(status: StatusCode) -> Result<()> {
    if status.is_success() {
        return Ok(());
    }
    let hint = match status.as_u16() {
        400 => "check the supplied argument values and request parameters",
        401 => "sign in again with joom auth <account> login <service-list>",
        403 => {
            "sign in for this service and check the app permissions and the resource is shared with this account"
        }
        404 => "check the resource ID and selected Microsoft service",
        429 => "Microsoft quota exceeded; retry later",
        300..=399 => "redirect refused",
        _ => "Microsoft request failed",
    };
    bail!("HTTP {}: {hint}", status.as_u16())
}

#[cfg(test)]
#[path = "../tests/unit/http.rs"]
mod tests;
