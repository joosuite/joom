use super::Request;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::ffi::OsString;

pub trait Backend {
    fn call(&mut self, service: &str, command: &str, params: Value) -> Result<Value>;
}

pub struct Joom {
    account: String,
    timeout: u64,
    store: crate::store::Store,
    http: crate::http::Http,
}

impl Joom {
    pub fn new(request: &Request) -> Result<Self> {
        let timeout = request.params["timeout"].as_u64().unwrap_or(30);
        Ok(Self {
            account: request.account.clone(),
            timeout,
            store: crate::store::Store::new(request.text("home"))?,
            http: crate::http::Http::new(timeout)?,
        })
    }
}

pub fn merge_pages(bytes: &[u8]) -> Result<Value> {
    let mut result = Value::Null;
    for page in serde_json::Deserializer::from_slice(bytes).into_iter::<Value>() {
        let page = page.context("joom returned invalid JSON")?;
        if result.is_null() {
            result = page;
        } else if let (Some(target), Some(page)) = (result.as_object_mut(), page.as_object()) {
            for (key, value) in page {
                ensure!(
                    target
                        .get(key)
                        .is_none_or(|previous| previous.is_array() == value.is_array()),
                    "joom returned an inconsistent paginated collection"
                );
                if let Some(rows) = target.get_mut(key).and_then(Value::as_array_mut) {
                    if let Some(more) = value.as_array() {
                        rows.extend(more.iter().cloned());
                    }
                } else {
                    target.insert(key.clone(), value.clone());
                }
            }
        } else {
            anyhow::bail!("unexpected paginated output from joom");
        }
    }
    if let Some(object) = result.as_object_mut() {
        object.remove("nextPageToken");
    }
    Ok(result)
}

impl Backend for Joom {
    fn call(&mut self, service: &str, command: &str, params: Value) -> Result<Value> {
        let file = tempfile::NamedTempFile::new()?;
        serde_json::to_writer(file.as_file(), &params)?;
        let mut args: Vec<OsString> = [
            "joom",
            "--json",
            "--timeout",
            &self.timeout.to_string(),
            service,
            &self.account,
            command,
            "--file",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.push(file.path().as_os_str().to_owned());
        let root = crate::cli::parse_from(args).context("invalid wrapper request")?;
        let definition = crate::services::SERVICES
            .iter()
            .find(|item| item.cli == service)
            .context("unknown Microsoft service")?;
        let (_, context) = root.subcommand().context("missing service")?;
        let call = crate::service_call(definition, context)?;
        let doc = &crate::schema_commands::contract(service).document;
        let mut bytes = Vec::new();
        crate::execute_with_output(
            &root,
            crate::native::arguments(context),
            (&self.store, &self.http),
            doc,
            call,
            &self.account,
            &mut |value, _| {
                serde_json::to_writer(&mut bytes, value)?;
                bytes.push(b'\n');
                ensure!(
                    bytes.len() as u64 <= crate::http::JSON_LIMIT,
                    "result exceeds 64 MiB; use a smaller limit:"
                );
                Ok(())
            },
        )
        .with_context(|| format!("{service} {command}"))?;
        merge_pages(&bytes)
    }
}
