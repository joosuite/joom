//! Explicit fixture setup and provider diagnostics for ignored live tests.
use super::*;
use reqwest::Method;
use serde_json::{Value, json};
use std::path::Path;

#[test]
#[ignore = "explicit live fixture or diagnostic operation; never signs in or refreshes"]
fn graph_fixture_operation() -> Result<()> {
    anyhow::ensure!(
        crate::http::live_session_only(),
        "live session-only mode required"
    );
    let input: Value =
        serde_json::from_slice(&std::fs::read(std::env::var("JOOM_LIVE_FIXTURE_REQUEST")?)?)?;
    let report = std::env::var("JOOM_LIVE_FIXTURE_REPORT")?;
    let account = input["account"].as_str().context("account required")?;
    let operation = input["operation"].as_str().context("operation required")?;
    let store = Store::new(None)?;
    let http = Http::new(20)?;
    let token = access_token(&store, &http, account)?;
    let mut diagnostics = Vec::new();
    let mut send = |method: Method,
                    path: &str,
                    body: Option<Value>,
                    prefer: Option<&str>|
     -> Result<Value> {
        let url = Url::parse(&format!("https://graph.microsoft.com/v1.0/{path}"))?;
        crate::microsoft::graph_url(&url)?;
        let mut request = http
            .client
            .request(method.clone(), url.clone())
            .bearer_auth(token.as_str());
        if let Some(body) = body {
            request = request.json(&body);
        }
        if let Some(prefer) = prefer {
            request = request.header("Prefer", prefer);
        }
        let response = request.send().map_err(|error| error.without_url())?;
        let status = response.status();
        let request_id = response
            .headers()
            .get("request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = limited(response, 1024 * 1024)?;
        let value: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)?
        };
        let message = value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .map(|message| {
                message
                    .replace(token.as_str(), "[redacted]")
                    .chars()
                    .filter(|ch| !ch.is_control())
                    .take(1000)
                    .collect::<String>()
            });
        diagnostics.push(json!({"method":method.as_str(),"path":url.path(),"status":status.as_u16(),"requestId":request_id,"code":value.pointer("/error/code"),"message":message}));
        anyhow::ensure!(
            status.is_success(),
            "fixture operation HTTP {}",
            status.as_u16()
        );
        Ok(value)
    };
    let result: Result<Value> = (|| match operation {
        "default-drive" => send(
            Method::GET,
            "me/drive?$select=id,driveType,webUrl",
            None,
            None,
        ),
        "drive-list" => send(
            Method::GET,
            "me/drives?$select=id,driveType,webUrl",
            None,
            None,
        ),
        "get-drive" => {
            let drive = input["driveId"].as_str().context("driveId required")?;
            let drive =
                percent_encoding::utf8_percent_encode(drive, percent_encoding::NON_ALPHANUMERIC);
            send(
                Method::GET,
                &format!("drives/{drive}?$select=id,driveType,webUrl"),
                None,
                None,
            )
        }
        "todo-status" => {
            let value = send(Method::GET, "me/todo/lists?$top=1", None, None)?;
            Ok(json!({"count":value["value"].as_array().map(Vec::len)}))
        }
        "mailbox-address" => {
            let calendar = send(Method::GET, "me/calendar?$select=owner", None, None)?;
            Ok(json!({"address":calendar.pointer("/owner/address")}))
        }
        "drive-search" => {
            let marker = input["marker"].as_str().context("marker required")?;
            anyhow::ensure!(
                marker.starts_with("joom-test-")
                    && marker
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
                "invalid fixture marker"
            );
            let suffix = if input["ordered"] == true {
                "&$orderby=lastModifiedDateTime%20desc"
            } else {
                ""
            };
            let value = send(
                Method::GET,
                &format!("me/drive/root/search(q='{marker}')?$top=2{suffix}"),
                None,
                None,
            )?;
            Ok(json!({"count":value["value"].as_array().map(Vec::len)}))
        }
        "create-recurring-event" => {
            let marker = input["marker"].as_str().context("marker required")?;
            anyhow::ensure!(marker.starts_with("joom-test-"), "invalid fixture marker");
            let calendar = input["calendarId"]
                .as_str()
                .context("calendarId required")?;
            let calendar =
                percent_encoding::utf8_percent_encode(calendar, percent_encoding::NON_ALPHANUMERIC);
            let value = send(Method::GET, &format!("me/calendars/{calendar}"), None, None)?;
            anyhow::ensure!(
                value["name"]
                    .as_str()
                    .is_some_and(|name| name.starts_with(marker)),
                "recurrence fixture requires the registered synthetic calendar"
            );
            let date = input["date"].as_str().context("date required")?;
            let body = json!({"subject":marker,"start":{"dateTime":format!("{date}T10:00:00"),"timeZone":"UTC"},"end":{"dateTime":format!("{date}T11:00:00"),"timeZone":"UTC"},"recurrence":{"pattern":{"type":"daily","interval":1},"range":{"type":"numbered","startDate":date,"numberOfOccurrences":2,"recurrenceTimeZone":"UTC"}}});
            let event = send(
                Method::POST,
                &format!("me/calendars/{calendar}/events"),
                Some(body),
                None,
            )?;
            Ok(
                json!({"id":event["id"],"type":event["type"],"mailboxAddress":value.pointer("/owner/address")}),
            )
        }
        "recycle-locked-fixture" => {
            let marker = input["marker"].as_str().context("marker required")?;
            anyhow::ensure!(marker.starts_with("joom-test-"), "invalid fixture marker");
            let item = input["fileId"].as_str().context("fileId required")?;
            let item =
                percent_encoding::utf8_percent_encode(item, percent_encoding::NON_ALPHANUMERIC);
            let metadata = send(
                Method::GET,
                &format!("me/drive/items/{item}?$select=id,name,eTag"),
                None,
                None,
            )?;
            anyhow::ensure!(
                metadata["name"]
                    .as_str()
                    .is_some_and(|name| name.starts_with(marker)),
                "refusing to recycle a non-fixture item"
            );
            send(
                Method::DELETE,
                &format!("me/drive/items/{item}"),
                None,
                Some("bypass-shared-lock"),
            )?;
            Ok(json!({"recycled":true}))
        }
        _ => bail!("unknown live fixture operation"),
    })();
    let record = json!({"account":account,"operation":operation,"result":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string),"requests":diagnostics});
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(Path::new(&report))?;
    serde_json::to_writer_pretty(&mut file, &record)?;
    file.write_all(b"\n")?;
    result.map(|_| ())
}
