use super::*;

#[test]
fn contract_urls_accept_only_the_pinned_graph_origin() {
    for path in ["me/messages", "me/drive", "me/todo/lists"] {
        assert!(
            contract_url(&Url::parse(&format!("https://graph.microsoft.com/v1.0/{path}")).unwrap())
                .is_ok()
        );
    }
    for url in [
        "http://graph.microsoft.com/v1.0/me",
        "https://graph.microsoft.com.attacker.test/v1.0/me",
        "https://attacker.graph.microsoft.com/v1.0/me",
        "https://user@graph.microsoft.com/v1.0/me",
        "https://graph.microsoft.com:444/v1.0/me",
        "https://graph.microsoft.com/v1.0/me#x",
        "https://graph.microsoft.com/beta/me",
    ] {
        assert!(contract_url(&Url::parse(url).unwrap()).is_err(), "{url}");
    }
}

#[test]
fn response_limit_detects_truncation() {
    assert_eq!(limited(&b"1234"[..], 4).unwrap(), b"1234");
    assert!(limited(&b"12345"[..], 4).is_err());
}

#[test]
fn redirects_are_refused_and_error_bodies_are_not_echoed() {
    use std::{io::Write, net::TcpListener};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0_u8; 2048];
        let n = stream.read(&mut buf).unwrap();
        assert!(buf[..n].starts_with(b"GET "));
        let body = "sensitive-reflected-data";
        write!(stream, "HTTP/1.1 302 Found\r\nLocation: http://{address}/other\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        listener
    });
    let error = Http::new(2)
        .unwrap()
        .json(
            &Method::GET,
            &Url::parse(&format!("http://{address}/")).unwrap(),
            None,
            None,
            1024,
        )
        .unwrap_err();
    assert!(error.to_string().contains("redirect refused"));
    assert!(!error.to_string().contains("sensitive-reflected-data"));
    let listener = server.join().unwrap();
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn bodyless_mutations_send_a_zero_content_length() {
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
    };
    for method in [Method::POST, Method::PUT, Method::PATCH] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line);
            }
            assert!(
                headers
                    .to_ascii_lowercase()
                    .contains("content-length: 0\r\n")
            );
            write!(
                stream,
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });
        assert!(
            Http::new(2)
                .unwrap()
                .json(
                    &method,
                    &Url::parse(&format!("http://{address}/")).unwrap(),
                    None,
                    None,
                    1024
                )
                .unwrap()
                .is_null()
        );
        server.join().unwrap();
    }
}

#[test]
fn writes_are_not_retried_on_service_failure() {
    use std::{io::Write, net::TcpListener};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0_u8; 2048];
        let n = stream.read(&mut buf).unwrap();
        assert!(buf[..n].starts_with(b"POST "));
        write!(
            stream,
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        listener
    });
    assert!(
        Http::new(2)
            .unwrap()
            .json(
                &Method::POST,
                &Url::parse(&format!("http://{address}/")).unwrap(),
                None,
                None,
                1024
            )
            .is_err()
    );
    let listener = server.join().unwrap();
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn adversarial_auth_failures_stop_after_one_request_without_echoing_the_body() {
    use std::{io::Write, net::TcpListener};
    for status in ["401 Unauthorized", "403 Forbidden"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 2048];
            assert!(stream.read(&mut request).unwrap() > 0);
            let body = "private-code<script>injected</script>";
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            listener
        });
        let error = Http::new(2)
            .unwrap()
            .json(
                &Method::GET,
                &Url::parse(&format!("http://{address}/")).unwrap(),
                None,
                None,
                1024,
            )
            .unwrap_err();
        assert!(error.to_string().contains(&status[..3]));
        assert!(!error.to_string().contains("private-code"));
        assert!(!error.to_string().contains("<script>"));
        let listener = server.join().unwrap();
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}
