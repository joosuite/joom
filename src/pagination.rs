use crate::discovery::MethodSpec;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Map, Value};
use std::collections::HashSet;

pub struct Plan {
    pub paginated: bool,
    maximum: Option<u32>,
    pages: Option<u32>,
    page_size: Option<(String, u32, u32)>,
}

impl Plan {
    pub fn new(
        spec: &MethodSpec,
        read: bool,
        maximum: Option<u32>,
        pages: Option<u32>,
    ) -> Result<Self> {
        let paginated = read
            && (spec.parameters.contains_key("pageToken")
                || spec.body_parameters.contains_key("pageToken"));
        ensure!(
            pages.is_none() || paginated,
            "--max-pages requires a paginated read method"
        );
        let page_size = ["pageSize", "maxResults"].into_iter().find_map(|name| {
            spec.parameters
                .get(name)
                .or_else(|| spec.body_parameters.get(name))
                .map(|parameter| {
                    (
                        name.to_owned(),
                        parameter
                            .minimum
                            .as_deref()
                            .and_then(|value| value.parse::<u32>().ok())
                            .unwrap_or(1),
                        parameter
                            .maximum
                            .as_deref()
                            .and_then(|value| value.parse::<u32>().ok())
                            .unwrap_or(100),
                    )
                })
        });
        let list = paginated
            || page_size.is_some()
            || spec.id.ends_with(".list")
            || spec.id.ends_with(".search");
        ensure!(
            maximum.is_none() || (read && list),
            "--max requires a read method returning a list"
        );
        Ok(Self {
            paginated,
            maximum,
            pages,
            page_size,
        })
    }
}

fn collection(value: &mut Value) -> Result<Option<&mut Vec<Value>>> {
    if value.is_array() {
        return Ok(value.as_array_mut());
    }
    let Some(object) = value.as_object_mut() else {
        return Ok(None);
    };
    let key = if object.get("items").is_some_and(Value::is_array) {
        Some("items".to_owned())
    } else {
        let mut arrays = object.iter().filter(|(_, value)| value.is_array());
        let first = arrays.next().map(|(key, _)| key.clone());
        ensure!(
            arrays.next().is_none(),
            "cannot apply --max to multiple result lists; select the result list with --fields"
        );
        first
    };
    Ok(key
        .and_then(|key| object.get_mut(&key))
        .and_then(Value::as_array_mut))
}

pub fn run(
    params: &mut Map<String, Value>,
    plan: Plan,
    mut fetch: impl FnMut(&Map<String, Value>) -> Result<Value>,
    mut emit: impl FnMut(&Value, bool) -> Result<()>,
) -> Result<()> {
    if plan.paginated
        && let Some(Value::String(fields)) = params.get_mut("fields")
        && !fields
            .split(',')
            .any(|field| matches!(field.trim(), "*" | "nextPageToken"))
    {
        fields.push_str(",nextPageToken");
    }
    let mut remaining = plan.maximum;
    let mut pages = 0_u64;
    let mut seen = HashSet::new();
    if let Some(Value::String(token)) = params.get("pageToken") {
        seen.insert(token.clone());
    }
    loop {
        if let (Some(remaining), Some((name, api_min, api_max))) = (remaining, &plan.page_size) {
            let requested = params
                .get(name)
                .and_then(|value| {
                    value
                        .as_u64()
                        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
                })
                .unwrap_or(u64::from(*api_max));
            params.insert(
                name.clone(),
                Value::from(
                    u64::from(remaining)
                        .min(u64::from(*api_max))
                        .min(requested)
                        .max(u64::from(*api_min)),
                ),
            );
        }
        let mut value = fetch(params)?;
        let next = value
            .get("nextPageToken")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .map(str::to_owned);
        if let Some(limit) = remaining {
            if let Some(rows) = collection(&mut value)? {
                rows.truncate(limit as usize);
                remaining = Some(limit - u32::try_from(rows.len()).context("too many results")?);
            } else if !plan.paginated && !value.is_null() {
                // An empty list response may omit its collection entirely.
                let empty = value.as_object().is_some_and(|object| {
                    object
                        .keys()
                        .all(|key| matches!(key.as_str(), "kind" | "etag" | "resultSizeEstimate"))
                });
                ensure!(
                    empty,
                    "--max requires list results; include the result list in --fields"
                );
            }
        }
        emit(&value, plan.paginated)?;
        pages += 1;
        if !plan.paginated
            || next.is_none()
            || remaining == Some(0)
            || plan
                .pages
                .is_some_and(|maximum| pages >= u64::from(maximum))
        {
            return Ok(());
        }
        let token = next.context("missing page token")?;
        ensure!(
            token.len() <= 8192,
            "Microsoft returned an oversized page token"
        );
        if !seen.insert(token.clone()) {
            bail!("Microsoft returned a repeated page token; output contains partial results");
        }
        params.insert("pageToken".into(), Value::String(token));
    }
}

pub fn request(
    spec: &MethodSpec,
    params: &mut Map<String, Value>,
    body: &mut Option<Value>,
    mut plan: Plan,
    mut fetch: impl FnMut(&Map<String, Value>, Option<&Value>) -> Result<Value>,
    emit: impl FnMut(&Value, bool) -> Result<()>,
) -> Result<()> {
    if spec
        .unpaginated_when
        .iter()
        .any(|key| params.contains_key(key))
    {
        plan.paginated = false;
        plan.page_size = None;
    }
    if spec.body_parameters.contains_key("pageToken") {
        if let Some(Value::String(fields)) = params.get_mut("fields")
            && !fields
                .split(',')
                .any(|field| matches!(field.trim(), "*" | "nextPageToken"))
        {
            fields.push_str(",nextPageToken");
        }
        let body = body
            .get_or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .context("pagination requires an object body")?;
        run(
            body,
            plan,
            |page| fetch(params, Some(&Value::Object(page.clone()))),
            emit,
        )
    } else {
        run(params, plan, |page| fetch(page, body.as_ref()), emit)
    }
}

#[cfg(test)]
#[path = "../tests/unit/pagination.rs"]
mod tests;
