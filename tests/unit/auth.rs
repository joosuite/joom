use super::*;

#[cfg(feature = "live-test-diagnostics")]
#[test]
#[ignore = "reads metadata from explicitly selected stored live sessions; never authenticates"]
fn stored_live_session_metadata() -> Result<()> {
    let accounts = std::env::var("JOOM_LIVE_ACCOUNTS")?;
    let store = Store::new(None)?;
    for account in accounts.split(',') {
        let tokens: Tokens = store.secret(&format!("account/{account}"))?;
        let claims = tokens
            .access_token
            .split('.')
            .nth(1)
            .and_then(|payload| URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok())
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let record = serde_json::json!({"account":account,"personal":tokens.personal,
            "expiresAt":tokens.expires_at,"validForSeconds":tokens.expires_at.saturating_sub(now()?),
            "audience":claims.as_ref().and_then(|value|value.get("aud")),
            "scopes":claims.as_ref().and_then(|value|value.get("scp"))});
        writeln!(std::io::stdout().lock(), "{record}")?;
    }
    Ok(())
}

#[cfg(feature = "live-test-diagnostics")]
#[test]
#[ignore = "one explicitly approved silent refresh; no browser sign-in or retries"]
fn refresh_stored_live_session_once() -> Result<()> {
    anyhow::ensure!(
        std::env::var("JOOM_LIVE_REFRESH_APPROVED").as_deref() == Ok("1"),
        "explicit refresh approval required"
    );
    let account = std::env::var("JOOM_LIVE_ACCOUNT")?;
    let store = Store::new(None)?;
    let http = Http::new(15)?;
    let mut tokens: Tokens = store.secret(&format!("account/{account}"))?;
    let mut credentials: Credentials = store.secret("oauth-client")?;
    credentials.validate()?;
    let scope = match std::env::var("JOOM_LIVE_REFRESH_SCOPES").as_deref() {
        Ok("all") => format!(
            "User.Read offline_access {}",
            crate::services::scopes("all")?.join(" ")
        ),
        Err(_) => String::new(),
        _ => bail!("refresh scope selection must be all or omitted"),
    };
    let mut fields = vec![
        ("client_id", credentials.client_id.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", tokens.refresh_token.as_str()),
    ];
    if !scope.is_empty() {
        fields.push(("scope", &scope));
    }
    let response = token_request(&http, true, &fields)?;
    let profile = http.json(
        &reqwest::Method::GET,
        &Url::parse("https://graph.microsoft.com/v1.0/me?$select=mail,userPrincipalName")?,
        None,
        Some(&response.access_token),
        1024 * 1024,
    )?;
    profile_account(&profile, Some(&account))?;
    tokens.access_token = response.access_token.clone();
    if !response.refresh_token.is_empty() {
        tokens.refresh_token = response.refresh_token.clone();
    }
    tokens.expires_at = now()?.saturating_add(response.expires_in);
    tokens.personal = true;
    store.set_secret(&format!("account/{account}"), &tokens)?;
    writeln!(
        std::io::stdout().lock(),
        "{}",
        serde_json::json!({"account":account,"refreshed":true,"personal":true,"scopes":response.scope,"expiresAt":tokens.expires_at})
    )?;
    Ok(())
}

#[test]
fn account_selection_checks_both_microsoft_profile_addresses() {
    let profile =
        serde_json::json!({"mail":"mailbox@outlook.com","userPrincipalName":"login@example.com"});
    assert_eq!(
        profile_account(&profile, Some("LOGIN@example.com")).unwrap(),
        "login@example.com"
    );
    assert_eq!(
        profile_account(&profile, Some("mailbox@outlook.com")).unwrap(),
        "mailbox@outlook.com"
    );
    assert_eq!(
        profile_account(&profile, None).unwrap(),
        "mailbox@outlook.com"
    );
    assert!(profile_account(&profile, Some("unverified-alias@example.com")).is_err());
    assert!(profile_account(&serde_json::json!({}), None).is_err());
}

#[test]
fn microsoft_callback_uses_localhost_and_the_bound_loopback_port() {
    let first = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let second = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    for listener in [&first, &second] {
        let redirect = redirect_uri(listener).unwrap();
        let redirect = Url::parse(&redirect).unwrap();
        assert_eq!(redirect.scheme(), "http");
        assert_eq!(redirect.host_str(), Some("localhost"));
        assert_eq!(redirect.port(), Some(listener.local_addr().unwrap().port()));
        assert_eq!(redirect.path(), "/oauth/callback");
    }
    assert_ne!(
        first.local_addr().unwrap().port(),
        second.local_addr().unwrap().port()
    );
    let external = TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    assert!(redirect_uri(&external).is_err());
}

#[test]
fn native_code_exchange_and_refresh_never_send_a_stored_client_secret() {
    for fields in [
        vec![
            ("grant_type", "authorization_code"),
            ("code", "synthetic-code"),
            ("code_verifier", "synthetic-verifier"),
            ("client_secret", "stored-secret"),
        ],
        vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", "synthetic-refresh"),
            ("client_secret", "stored-secret"),
        ],
    ] {
        let public = public_client_fields(&fields);
        assert!(!public.iter().any(|(key, _)| *key == "client_secret"));
        assert_eq!(public.len(), fields.len() - 1);
        assert_eq!(public[0], fields[0]);
    }
}

#[test]
fn localhost_callback_reaches_the_loopback_listener() {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let redirect = Url::parse(&redirect_uri(&listener).unwrap()).unwrap();
    let port = redirect.port().unwrap();
    let callback = std::thread::spawn(move || {
        wait_callback(&listener, "synthetic-state", Duration::from_secs(3))
    });
    let mut stream = TcpStream::connect(("localhost", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(stream,"GET /oauth/callback?state=synthetic-state&code=synthetic-code HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert_eq!(callback.join().unwrap().unwrap(), "synthetic-code");
}

#[test]
fn enrollment_requires_an_application_id_and_normalizes_pasted_whitespace() {
    let mut credentials: Credentials = serde_json::from_value(
        serde_json::json!({"client_id":" 00001111-aaaa-2222-bbbb-3333cccc4444\n"}),
    )
    .unwrap();
    credentials.validate().unwrap();
    assert_eq!(
        credentials.client_id,
        "00001111-aaaa-2222-bbbb-3333cccc4444"
    );
    assert!(credentials.client_secret.is_empty());
    for invalid in [
        "",
        "not-an-application-id.example.com",
        "client-secret-value",
        "00000000-0000-0000-0000-000000000000",
    ] {
        credentials.client_id = invalid.into();
        let error = credentials.validate().unwrap_err().to_string();
        assert!(error.contains("Application (client) ID"));
        if !invalid.is_empty() {
            assert!(!error.contains(invalid));
        }
    }
}

#[test]
fn a_malformed_refresh_token_cannot_verify_client_registration() {
    // Microsoft can reject the fake refresh token before looking up the client.
    // This is a connectivity result only, never proof of client or audience validity.
    assert!(
        client_test_response(
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"error":"invalid_grant","error_codes":[9002313]}"#
        )
        .is_ok()
    );
    let credentials = Credentials {
        client_id: "00001111-aaaa-2222-bbbb-3333cccc4444".into(),
        client_secret: String::new(),
    };
    let request =
        client_test_request(&Http::new(2).unwrap(), &credentials, "synthetic-probe").unwrap();
    let fields = url::form_urlencoded::parse(request.body().unwrap().as_bytes().unwrap())
        .collect::<std::collections::HashMap<_, _>>();
    assert!(!fields.contains_key("client_secret"));
}

#[test]
fn connectivity_probe_posts_client_credentials_only_to_microsoft() {
    let credentials = Credentials {
        client_id: "dummy-client-id".into(),
        client_secret: "dummy-client-secret".into(),
    };
    let request = client_test_request(&Http::new(2).unwrap(), &credentials, "dummy-probe").unwrap();
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(request.url().as_str(), TOKEN);
    assert!(request.url().query().is_none());
    assert!(
        !request
            .headers()
            .contains_key(reqwest::header::AUTHORIZATION)
    );
    let fields = url::form_urlencoded::parse(request.body().unwrap().as_bytes().unwrap())
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields["client_id"], "dummy-client-id");
    assert!(!fields.contains_key("client_secret"));
    assert_eq!(fields["grant_type"], "refresh_token");
    assert_eq!(fields["refresh_token"], "dummy-probe");
}

#[test]
fn connectivity_probe_accepts_only_the_expected_microsoft_response() {
    use reqwest::StatusCode;
    assert!(client_test_response(StatusCode::BAD_REQUEST, br#"{"error":"invalid_grant"}"#).is_ok());
    for (status, body) in [
        (StatusCode::UNAUTHORIZED, r#"{"error":"invalid_client"}"#),
        (StatusCode::BAD_REQUEST, r#"{"error":"deleted_client"}"#),
        (
            StatusCode::BAD_REQUEST,
            r#"{"error":"unauthorized_client"}"#,
        ),
        (StatusCode::BAD_REQUEST, r#"{"error":"invalid_request"}"#),
        (StatusCode::BAD_REQUEST, "not JSON"),
        (StatusCode::BAD_REQUEST, "{}"),
        (StatusCode::UNAUTHORIZED, r#"{"error":"invalid_grant"}"#),
        (StatusCode::OK, r#"{"access_token":"must-not-be-used"}"#),
        (StatusCode::FOUND, r#"{"error":"invalid_grant"}"#),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            r#"{"error":"invalid_grant"}"#,
        ),
    ] {
        assert!(
            client_test_response(status, body.as_bytes()).is_err(),
            "{status}: {body}"
        );
    }
}

#[test]
fn connectivity_probe_errors_never_echo_remote_fields_or_secrets() {
    for error in [
        "invalid_client",
        "deleted_client",
        "unauthorized_client",
        "reflected-secret",
    ] {
        let body = serde_json::json!({"error":error,"error_description":"reflected-secret","client_secret":"reflected-secret"});
        let result = client_test_response(
            reqwest::StatusCode::BAD_REQUEST,
            &serde_json::to_vec(&body).unwrap(),
        )
        .unwrap_err();
        assert!(!format!("{result:#}").contains("reflected-secret"));
        if error != "reflected-secret" {
            assert!(result.to_string().contains("joom auth enroll"));
        }
    }
}

#[test]
fn browser_authorization_uses_pkce_s256_and_account_hint() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let redirect = "http://localhost:49152/oauth/callback";
    let url = authorization_url(
        "client-id",
        redirect,
        vec!["Mail.Read".into(), "Mail.Read".into()],
        Some("you@example.com"),
        verifier,
        "random-state",
        false,
    )
    .unwrap();
    assert_eq!(
        url.origin().ascii_serialization(),
        "https://login.microsoftonline.com"
    );
    assert_eq!(url.path(), "/common/oauth2/v2.0/authorize");
    let pairs = url
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(pairs["code_challenge_method"], "S256");
    assert_eq!(
        pairs["code_challenge"],
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    assert_eq!(pairs["redirect_uri"], redirect);
    assert_eq!(pairs["response_type"], "code");
    assert_eq!(pairs["state"], "random-state");
    assert_eq!(pairs["login_hint"], "you@example.com");
    assert!(!pairs.contains_key("access_type"));
    assert_eq!(
        pairs["scope"],
        "Mail.Read User.Read email offline_access openid"
    );
    assert!(!pairs.contains_key("code_verifier"));
    assert!(!pairs.contains_key("client_secret"));
}

#[test]
fn personal_sign_in_uses_consumers_without_guessing_from_the_email_domain() {
    for personal in [false, true] {
        let url = authorization_url(
            "client-id",
            "http://localhost:49152/oauth/callback",
            vec!["User.Read".into()],
            Some("person@gmail.com"),
            "verifier",
            "state",
            personal,
        )
        .unwrap();
        let tenant = if personal { "consumers" } else { "common" };
        assert_eq!(url.path(), format!("/{tenant}/oauth2/v2.0/authorize"));
        assert_eq!(
            oauth_endpoint(personal, "token"),
            format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token")
        );
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "login_hint" && value == "person@gmail.com")
        );
    }
    let legacy: Tokens = serde_json::from_value(serde_json::json!({
        "access_token": "old-access", "refresh_token": "old-refresh", "expires_at": 1
    }))
    .unwrap();
    assert!(!legacy.personal);
    let personal: Tokens = serde_json::from_value(serde_json::json!({
        "access_token": "access", "refresh_token": "refresh", "expires_at": 1,
        "personal": true
    }))
    .unwrap();
    assert!(personal.personal);
}

#[test]
fn browser_launcher_passes_url_as_one_argument_without_shell() {
    let url =
        Url::parse("https://login.microsoftonline.com/common/oauth2/v2.0/authorize?state=x&scope=email%20openid")
            .unwrap();
    let command = browser_command(&url);
    let args = command.get_args().collect::<Vec<_>>();
    #[cfg(target_os = "macos")]
    assert_eq!(command.get_program(), "/usr/bin/open");
    #[cfg(target_os = "windows")]
    {
        assert_eq!(command.get_program(), "rundll32.exe");
        assert_eq!(args[0], "url.dll,FileProtocolHandler");
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    assert_eq!(command.get_program(), "xdg-open");
    assert_eq!(args.len(), if cfg!(target_os = "windows") { 2 } else { 1 });
    assert_eq!(args.last().unwrap(), &url.as_str());
}

#[test]
fn callback_requires_exact_state_and_single_code() {
    assert_eq!(
        callback_code("/oauth/callback?state=abc&code=xyz", "abc").unwrap(),
        "xyz"
    );
    for target in [
        "/oauth/callback?state=bad&code=xyz",
        "/oauth/callback?state=abc&code=x&code=y",
        "/oauth/callback?state=abc&state=abc&code=x",
        "/oauth/callback?state=abc&error=denied",
        "//evil.test/?state=abc&code=x",
        "/oauth/callback?state=abc&code=",
    ] {
        assert!(callback_code(target, "abc").is_err(), "{target}");
    }
}

#[test]
fn callback_accepts_a_browser_request_delivered_in_fragments() {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let browser = std::thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream.write_all(b"GET /oauth/callback?state=expected-state&iss=https%3A%2F%2Flogin.microsoftonline.com&code=dummy-code&scope=email%20openid&session_state=example-session HTTP/1.1\r\nHost: 127.0.0.1\r\n").unwrap();
        std::thread::sleep(Duration::from_millis(100));
        stream.write_all(b"Connection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    });
    let result = wait_callback(&listener, "expected-state", Duration::from_secs(2));
    let response = browser.join().unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains("Content-Type: text/html; charset=utf-8"));
    assert!(response.contains("You're done here."));
    assert!(response.contains("You can close this tab and return to your terminal."));
    assert!(response.contains("Joom will confirm in your terminal when sign-in is complete."));
    assert!(!response.contains("dummy-code"));
    assert!(!response.contains("expected-state"));
    assert_eq!(result.unwrap(), "dummy-code");
}

fn browser_callback(address: std::net::SocketAddr, target: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(
        stream,
        "GET {target} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn callback_keeps_waiting_after_unrelated_or_wrong_state_requests() {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let browser = std::thread::spawn(move || {
        let unrelated = browser_callback(address, "/favicon.ico");
        assert!(unrelated.starts_with("HTTP/1.1 404 Not Found"));
        let wrong_state = browser_callback(
            address,
            "/oauth/callback?state=private-state&code=private-code",
        );
        assert!(wrong_state.starts_with("HTTP/1.1 400 Bad Request"));
        assert!(wrong_state.contains("does not match the current sign-in attempt"));
        assert!(!wrong_state.contains("private-state"));
        assert!(!wrong_state.contains("private-code"));
        let valid = browser_callback(
            address,
            "/oauth/callback?state=current-state&code=4%2Fdummy-code&iss=https%3A%2F%2Flogin.microsoftonline.com&scope=email%20openid",
        );
        assert!(valid.starts_with("HTTP/1.1 200 OK"));
    });
    let code = wait_callback(&listener, "current-state", Duration::from_secs(3)).unwrap();
    browser.join().unwrap();
    assert_eq!(code, "4/dummy-code");
}

#[test]
fn callback_reports_microsoft_denial_instead_of_waiting_until_timeout() {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let browser = std::thread::spawn(move || {
        browser_callback(
            address,
            "/oauth/callback?state=current-state&error=access_denied&error_description=private-error",
        )
    });
    let error = wait_callback(&listener, "current-state", Duration::from_secs(3)).unwrap_err();
    let response = browser.join().unwrap();
    assert!(matches!(
        error.downcast_ref::<CallbackError>(),
        Some(CallbackError::AuthorizationDenied)
    ));
    assert!(response.starts_with("HTTP/1.1 403 Forbidden"));
    assert!(response.contains("Microsoft did not authorize sign-in"));
    assert!(!response.contains("private-error"));
    assert!(!error.to_string().contains("private-error"));
}

#[test]
fn verifier_has_rfc7636_length_and_entropy() {
    let first = random_string().unwrap();
    assert_eq!(first.len(), 43);
    assert_ne!(first, random_string().unwrap());
}
