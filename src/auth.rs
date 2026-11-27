use crate::{
    http::{Http, check_status, limited},
    store::Store,
};
use anyhow::{Context, Result, bail};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const TOKEN: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";

pub mod transfer;

#[cfg(all(test, feature = "live-test-diagnostics"))]
#[path = "../tests/live/fixtures.rs"]
mod live;

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Credentials {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

impl Credentials {
    fn validate(&mut self) -> Result<()> {
        self.client_id = self.client_id.trim().to_owned();
        let id = self.client_id.as_bytes();
        anyhow::ensure!(
            id.len() == 36
                && id.iter().enumerate().all(|(index, byte)| {
                    if [8, 13, 18, 23].contains(&index) {
                        *byte == b'-'
                    } else {
                        byte.is_ascii_hexdigit()
                    }
                })
                && id
                    .iter()
                    .any(|byte| matches!(*byte,b'1'..=b'9'|b'a'..=b'f'|b'A'..=b'F')),
            "client_id must be the Microsoft Entra Application (client) ID in GUID format; do not use a directory ID, object ID, or client secret. See docs/authentication.md"
        );
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Tokens {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
    #[serde(default)]
    personal: bool,
}

fn oauth_endpoint(personal: bool, operation: &str) -> String {
    let tenant = if personal { "consumers" } else { "common" };
    format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/{operation}")
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    expires_in: u64,
    token_type: String,
    #[serde(default)]
    scope: String,
}

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn masked_prompt(prompt: &str) -> std::io::Result<String> {
    rpassword::prompt_password_with_config(
        prompt,
        rpassword::ConfigBuilder::new()
            .password_feedback_mask('*')
            .build(),
    )
}

pub fn enroll(store: &Store, stdin: bool) -> Result<()> {
    let mut creds: Credentials = if stdin {
        let raw = Zeroizing::new(limited(std::io::stdin().lock(), 1024 * 1024)?);
        serde_json::from_slice(&raw).context("expected Microsoft client_id JSON on stdin")?
    } else {
        let mut creds = Credentials {
            client_id: String::new(),
            client_secret: String::new(),
        };
        creds.client_id = masked_prompt("Microsoft client ID: ")?;
        creds
    };
    creds.validate()?;
    // Retain deserialization compatibility with older credential files, while
    // enrolling this native PKCE client using only its application ID.
    creds.client_secret.zeroize();
    store.initialize()?;
    store.set_secret("oauth-client", &creds)
}

pub fn test(store: &Store, http: &Http) -> Result<()> {
    let mut creds: Credentials = store
        .secret("oauth-client")
        .context("enroll your Microsoft Application (client) ID with joom auth enroll")?;
    creds.validate()?;
    let probe = Zeroizing::new(format!("joom-connectivity-test-{}", random_string()?));
    let request = client_test_request(http, &creds, &probe)?;
    let response = http
        .client
        .execute(request)
        .map_err(|e| e.without_url())
        .context("cannot reach Microsoft's OAuth token endpoint")?;
    let status = response.status();
    let bytes = Zeroizing::new(limited(response, 1024 * 1024)?);
    client_test_response(status, &bytes)
}

fn client_test_request(
    http: &Http,
    creds: &Credentials,
    probe: &str,
) -> Result<reqwest::blocking::Request> {
    Ok(http
        .client
        .post(TOKEN)
        .form(&[
            ("client_id", creds.client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", probe),
        ])
        .build()?)
}

fn client_test_response(status: reqwest::StatusCode, bytes: &[u8]) -> Result<()> {
    #[derive(Deserialize)]
    struct OAuthError {
        error: String,
    }
    if !matches!(status.as_u16(), 400 | 401) {
        check_status(status)?;
        bail!("Microsoft returned an unexpected response to the connectivity test");
    }
    let response: OAuthError = serde_json::from_slice(bytes)
        .context("Microsoft returned an invalid OAuth connectivity-test response")?;
    match (status.as_u16(), response.error.as_str()) {
        (400, "invalid_grant") => Ok(()),
        (_, "invalid_client") => {
            bail!(
                "Microsoft rejected the client ID or secret. Run 'joom auth enroll' to replace them."
            )
        }
        (_, "deleted_client") => bail!(
            "The Microsoft OAuth client has been deleted. Enroll an active mobile and desktop app registration with 'joom auth enroll'."
        ),
        (_, "unauthorized_client") => bail!(
            "Microsoft rejected the app registration or account type. Check the Application (client) ID, supported account types, and mobile/desktop callback registration; run 'joom auth enroll' if the ID is wrong."
        ),
        _ => bail!(
            "Microsoft is reachable but did not accept the OAuth connectivity test; check the enrolled client configuration"
        ),
    }
}

pub fn login(
    store: &Store,
    http: &Http,
    scopes: Vec<String>,
    account: Option<&str>,
    personal: bool,
) -> Result<String> {
    let mut creds: Credentials = store
        .secret("oauth-client")
        .context("enroll your Microsoft Application (client) ID with joom auth enroll")?;
    creds.validate()?;
    anyhow::ensure!(
        !scopes.is_empty(),
        "specify at least one service after login"
    );
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let redirect = redirect_uri(&listener)?;
    let verifier = Zeroizing::new(random_string()?);
    let state = Zeroizing::new(random_string()?);
    let url = authorization_url(
        &creds.client_id,
        &redirect,
        scopes,
        account,
        &verifier,
        &state,
        personal,
    )?;
    open_browser(&url)?;
    writeln!(
        std::io::stderr().lock(),
        "Complete sign-in in your browser. Waiting for authorization for up to 3 minutes."
    )?;
    let code = Zeroizing::new(wait_callback(&listener, &state, Duration::from_secs(180))?);
    let response = token_request(
        http,
        personal,
        &[
            ("client_id", &creds.client_id),
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect),
            ("code_verifier", &verifier),
        ],
    )?;
    anyhow::ensure!(
        !response.refresh_token.is_empty(),
        "Microsoft did not return a refresh token; retry consent"
    );
    let profile = http.json(
        &reqwest::Method::GET,
        &Url::parse("https://graph.microsoft.com/v1.0/me?$select=mail,userPrincipalName")?,
        None,
        Some(&response.access_token),
        1024 * 1024,
    )?;
    let email = profile_account(&profile, account)?;
    let tokens = Tokens {
        access_token: response.access_token.clone(),
        refresh_token: response.refresh_token.clone(),
        expires_at: now()?.saturating_add(response.expires_in),
        personal,
    };
    store.set_secret(&format!("account/{email}"), &tokens)?;
    let mut config = store.config()?;
    if !config.accounts.contains(&email) {
        config.accounts.push(email.clone());
    }
    config.account = Some(email.clone());
    store.save_config(&config)?;
    Ok(email)
}

fn profile_account(profile: &serde_json::Value, expected: Option<&str>) -> Result<String> {
    let addresses = ["mail", "userPrincipalName"]
        .into_iter()
        .filter_map(|key| profile[key].as_str().filter(|value| !value.is_empty()))
        .collect::<Vec<_>>();
    let primary = addresses
        .first()
        .context("Microsoft did not return an email address")?;
    if let Some(expected) = expected {
        anyhow::ensure!(
            addresses
                .iter()
                .any(|address| address.eq_ignore_ascii_case(expected)),
            "Microsoft returned a different account address. Use an address returned by the Microsoft profile, usually the primary account address; aliases absent from that profile cannot be verified automatically"
        );
        return Ok(expected.to_ascii_lowercase());
    }
    Ok(primary.to_ascii_lowercase())
}

fn redirect_uri(listener: &TcpListener) -> Result<String> {
    let address = listener.local_addr()?;
    anyhow::ensure!(
        address.ip().is_loopback(),
        "OAuth callback listener must bind to loopback"
    );
    // Microsoft ignores ephemeral ports when matching registered localhost URIs.
    // IP-literal callback ports do not receive that matching exception.
    Ok(format!(
        "http://localhost:{}/oauth/callback",
        address.port()
    ))
}

fn authorization_url(
    client_id: &str,
    redirect: &str,
    scopes: Vec<String>,
    account: Option<&str>,
    verifier: &str,
    state: &str,
    personal: bool,
) -> Result<Url> {
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = Url::parse(&oauth_endpoint(personal, "authorize"))?;
    let mut scopes = scopes;
    scopes.push("openid".into());
    scopes.push("email".into());
    scopes.push("offline_access".into());
    scopes.push("User.Read".into());
    scopes.sort();
    scopes.dedup();
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("scope", &scopes.join(" ")),
        ("prompt", "consent"),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
    ]);
    if let Some(account) = account {
        url.query_pairs_mut().append_pair("login_hint", account);
    }
    Ok(url)
}

fn browser_command(url: &Url) -> Command {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("/usr/bin/open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");
    command
        .arg(url.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn open_browser(url: &Url) -> Result<()> {
    let status = browser_command(url).status().context(
        "could not open the default browser; check your browser configuration and retry login",
    )?;
    anyhow::ensure!(
        status.success(),
        "could not open the default browser; check your browser configuration and retry login"
    );
    Ok(())
}

pub fn access_token(store: &Store, http: &Http, account: &str) -> Result<Zeroizing<String>> {
    let account = account.to_ascii_lowercase();
    let mut tokens: Tokens = store.secret(&format!("account/{account}"))?;
    if tokens.expires_at > now()?.saturating_add(60) {
        return Ok(Zeroizing::new(tokens.access_token.clone()));
    }
    anyhow::ensure!(
        !crate::http::live_session_only(),
        "live test stopped: stored token expired; no authentication or refresh attempted"
    );
    let creds: Credentials = store.secret("oauth-client")?;
    let response = token_request(
        http,
        tokens.personal,
        &[
            ("client_id", &creds.client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", &tokens.refresh_token),
        ],
    )?;
    tokens.access_token = response.access_token.clone();
    if !response.refresh_token.is_empty() {
        tokens.refresh_token = response.refresh_token.clone();
    }
    tokens.expires_at = now()?.saturating_add(response.expires_in);
    store.set_secret(&format!("account/{account}"), &tokens)?;
    Ok(Zeroizing::new(tokens.access_token.clone()))
}

pub fn logout(store: &Store, account: Option<&str>) -> Result<String> {
    let mut config = store.config()?;
    let email = account
        .or(config.account.as_deref())
        .context("choose an account with joom auth <account> logout")?
        .to_ascii_lowercase();
    store.delete_secret(&format!("account/{email}"))?;
    config.accounts.retain(|a| a != &email);
    if config.account.as_deref() == Some(&email) {
        config.account = config.accounts.first().cloned();
    }
    store.save_config(&config)?;
    Ok(email)
}

fn public_client_fields<'a>(fields: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    fields
        .iter()
        .copied()
        .filter(|(key, _)| *key != "client_secret")
        .collect()
}

fn token_request(http: &Http, personal: bool, fields: &[(&str, &str)]) -> Result<TokenResponse> {
    let response = http
        .client
        .post(oauth_endpoint(personal, "token"))
        .form(&public_client_fields(fields))
        .send()
        .map_err(|e| e.without_url())?;
    check_status(response.status())?;
    let bytes = Zeroizing::new(limited(response, 1024 * 1024)?);
    let tokens: TokenResponse =
        serde_json::from_slice(&bytes).context("invalid OAuth token response")?;
    anyhow::ensure!(
        tokens.token_type.eq_ignore_ascii_case("bearer") && !tokens.access_token.is_empty(),
        "invalid OAuth token type"
    );
    Ok(tokens)
}

fn random_string() -> Result<String> {
    let mut bytes = Zeroizing::new([0_u8; 32]);
    getrandom::fill(bytes.as_mut()).context("operating system randomness unavailable")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes.as_ref()))
}

#[derive(Debug)]
enum CallbackError {
    UnexpectedPath,
    InvalidState,
    AuthorizationDenied,
    MissingCode,
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnexpectedPath => "unexpected callback path",
            Self::InvalidState => "this callback does not match the current sign-in attempt",
            Self::AuthorizationDenied => "Microsoft did not authorize sign-in; run 'joom auth ACCOUNT login SERVICE_LIST' to try again",
            Self::MissingCode => "Microsoft did not return a single authorization code",
        })
    }
}

impl std::error::Error for CallbackError {}

fn callback_page(title: &str, message: &str, success: bool) -> String {
    include_str!("auth/callback.html")
        .replace(
            "{{brandLogo}}",
            &STANDARD.encode(include_bytes!("../assets/brand/logo.png")),
        )
        .replace(
            "{{brandIcon}}",
            &STANDARD.encode(include_bytes!("../assets/brand/icon.png")),
        )
        .replace("{{title}}", title)
        .replace("{{message}}", message)
        .replace("{{state}}", if success { "success" } else { "error" })
        .replace(
            "{{icon}}",
            if success {
                "M17 28l7 7 15-15"
            } else {
                "M28 16v15M28 39v1"
            },
        )
        .replace(
            "{{next}}",
            if success {
                "You can close this tab and return to your terminal."
            } else {
                "Return to your terminal to continue."
            },
        )
        .replace(
            "{{detail}}",
            if success {
                "Joom will confirm in your terminal when sign-in is complete."
            } else {
                "You can close this tab."
            },
        )
}

fn wait_callback(listener: &TcpListener, state: &str, timeout: Duration) -> Result<String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                // macOS inherits O_NONBLOCK from the listener. Browser headers can
                // arrive in several reads; wait for them within the socket timeout.
                stream.set_nonblocking(false)?;
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                let result = read_callback(&mut stream, state);
                let rejection = result
                    .as_ref()
                    .err()
                    .and_then(|e| e.downcast_ref::<CallbackError>());
                let (status, title, message) = match (&result, rejection) {
                    (Ok(_), _) => (
                        "200 OK",
                        "You're done here.",
                        "Microsoft has sent your approval to Joom.",
                    ),
                    (_, Some(CallbackError::UnexpectedPath)) => (
                        "404 Not Found",
                        "Nothing to sign in to here.",
                        "This address is not a sign-in callback.",
                    ),
                    (_, Some(CallbackError::InvalidState)) => (
                        "400 Bad Request",
                        "Let's use the right tab.",
                        "This callback does not match the current sign-in attempt. Complete sign-in in the browser opened by the current command.",
                    ),
                    (_, Some(CallbackError::AuthorizationDenied)) => (
                        "403 Forbidden",
                        "Sign-in wasn't approved.",
                        "Microsoft did not authorize sign-in. Return to the terminal to try again.",
                    ),
                    (_, Some(CallbackError::MissingCode)) => (
                        "400 Bad Request",
                        "Sign-in needs another try.",
                        "Microsoft did not return an authorization code. Return to the terminal to try again.",
                    ),
                    _ => (
                        "400 Bad Request",
                        "We couldn't finish this step.",
                        "The browser request could not be read. Return to the terminal for details.",
                    ),
                };
                let page = callback_page(title, message, result.is_ok());
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; img-src data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{page}",
                    page.len()
                );
                match result {
                    Ok(code) => return Ok(code),
                    Err(error) => match error.downcast_ref::<CallbackError>() {
                        Some(CallbackError::UnexpectedPath) => {}
                        Some(CallbackError::AuthorizationDenied) => return Err(error),
                        _ => writeln!(
                            std::io::stderr().lock(),
                            "Sign-in callback rejected: {error}. Still waiting for the current sign-in."
                        )?,
                    },
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(e) => return Err(e.into()),
        }
    }
    bail!("browser authorization timed out; run joom auth <account> login <service-list> again")
}

fn read_callback(stream: &mut TcpStream, state: &str) -> Result<String> {
    let mut bytes = Zeroizing::new(Vec::new());
    let start = Instant::now();
    while bytes.len() < 8192 && start.elapsed() < Duration::from_secs(2) {
        let mut buf = [0_u8; 512];
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    anyhow::ensure!(
        bytes.len() < 8192 && bytes.windows(4).any(|w| w == b"\r\n\r\n"),
        "invalid callback headers"
    );
    let request = std::str::from_utf8(&bytes)?;
    let mut words = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    anyhow::ensure!(words.next() == Some("GET"), "callback requires GET");
    callback_code(words.next().context("missing callback target")?, state)
}

fn callback_code(target: &str, state: &str) -> Result<String> {
    anyhow::ensure!(
        target.starts_with("/oauth/callback?"),
        CallbackError::UnexpectedPath
    );
    let url = Url::parse(&format!("http://127.0.0.1{target}"))?;
    anyhow::ensure!(
        url.path() == "/oauth/callback" && url.fragment().is_none(),
        CallbackError::UnexpectedPath
    );
    let pairs = url.query_pairs().collect::<Vec<_>>();
    let states = pairs
        .iter()
        .filter(|(k, _)| k == "state")
        .collect::<Vec<_>>();
    anyhow::ensure!(
        states.len() == 1 && bool::from(states[0].1.as_bytes().ct_eq(state.as_bytes())),
        CallbackError::InvalidState
    );
    anyhow::ensure!(
        !pairs.iter().any(|(k, _)| k == "error"),
        CallbackError::AuthorizationDenied
    );
    let codes = pairs
        .iter()
        .filter(|(k, _)| k == "code")
        .collect::<Vec<_>>();
    anyhow::ensure!(
        codes.len() == 1 && !codes[0].1.is_empty(),
        CallbackError::MissingCode
    );
    Ok(codes[0].1.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/auth.rs"]
mod tests;
