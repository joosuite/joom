use crate::http::contract_url;
use anyhow::{Context, Result, bail};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use url::Url;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub repeated: bool,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default, rename = "enum")]
    pub choices: Vec<String>,
    #[serde(default)]
    pub minimum: Option<String>,
    #[serde(default)]
    pub maximum: Option<String>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodSpec {
    #[serde(
        default,
        rename = "x-body-path-parameters",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub body_path_parameters: Vec<String>,
    #[serde(default, rename = "x-anonymous")]
    pub anonymous: bool,
    #[serde(default, rename = "x-base-url")]
    pub base_url: Option<String>,
    #[serde(default, rename = "x-unpaginated-when")]
    pub unpaginated_when: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cli_inputs: BTreeMap<String, Value>,
    #[serde(default, rename = "x-batch-operation")]
    pub batch_operation: Option<String>,
    #[serde(default, rename = "x-operation-request")]
    pub operation_request: Option<Value>,
    #[serde(default, rename = "x-batch-fields")]
    pub batch_fields: BTreeMap<String, Value>,
    #[serde(default, rename = "x-fresh-response")]
    pub fresh_response: bool,
    #[serde(default, rename = "x-read-only")]
    pub read_only: bool,
    #[serde(default, rename = "x-body-parameters")]
    pub body_parameters: BTreeMap<String, Parameter>,
    #[serde(default)]
    pub id: String,
    pub path: String,
    pub http_method: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, Parameter>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub request: Option<Value>,
    #[serde(default)]
    pub response: Option<Value>,
    #[serde(default)]
    pub supports_media_download: bool,
    #[serde(default)]
    pub media_upload: Option<Value>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Resource {
    #[serde(default)]
    pub methods: BTreeMap<String, MethodSpec>,
    #[serde(default)]
    pub resources: BTreeMap<String, Resource>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub root_url: String,
    #[serde(default)]
    pub service_path: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, Parameter>,
    #[serde(default)]
    pub schemas: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub root: Resource,
}

impl Document {
    pub fn method(&self, name: &str) -> Result<&MethodSpec> {
        if let Some(method) = self.root.methods.get(name) {
            return Ok(method);
        }
        fn by_path<'a>(mut resource: &'a Resource, path: &str) -> Option<&'a MethodSpec> {
            let mut parts = path.split('.').peekable();
            while let Some(part) = parts.next() {
                if parts.peek().is_none() {
                    return resource.methods.get(part);
                }
                resource = resource.resources.get(part)?;
            }
            None
        }
        // A resource may share its API's name, as in Forms' forms.get.
        if let Some(method) = by_path(&self.root, name) {
            return Ok(method);
        }
        if let Some(short) = name.strip_prefix(&format!("{}.", self.name))
            && let Some(method) = by_path(&self.root, short)
        {
            return Ok(method);
        }
        // Some Discovery IDs differ from their resource paths.
        fn find<'a>(resource: &'a Resource, id: &str) -> Option<&'a MethodSpec> {
            resource
                .methods
                .values()
                .find(|m| m.id == id)
                .or_else(|| resource.resources.values().find_map(|r| find(r, id)))
        }
        find(&self.root, name)
            .or_else(|| find(&self.root, &format!("{}.{}", self.name, name)))
            .with_context(|| {
                format!(
                    "unknown method {name}; check the method name for {} {}",
                    self.name, self.version
                )
            })
    }

    pub fn url(&self, method: &MethodSpec, params: &Map<String, Value>) -> Result<Url> {
        self.url_with_body(method, params, None)
    }

    pub fn url_with_body(
        &self,
        method: &MethodSpec,
        params: &Map<String, Value>,
        body: Option<&Value>,
    ) -> Result<Url> {
        let mut schemas = self.parameters.clone();
        schemas.extend(method.parameters.clone());
        let mut path = method.path.clone();
        for name in &method.body_path_parameters {
            let value = body
                .and_then(|body| body.get(name))
                .and_then(Value::as_str)
                .with_context(|| format!("missing request body path parameter {name}"))?;
            anyhow::ensure!(
                !value.is_empty(),
                "empty request body path parameter {name}"
            );
            path = path.replace(&format!("{{{name}}}"), &escape_segment(value)?);
        }
        anyhow::ensure!(!path.contains(['?', '#', '\\']), "invalid method path");
        let mut query = Vec::new();
        for key in params.keys() {
            anyhow::ensure!(
                key != "access_token" && key != "oauth_token",
                "bearer tokens come from the encrypted vault, not query parameters"
            );
            anyhow::ensure!(schemas.contains_key(key), "unknown parameter {key}");
        }
        for (name, spec) in schemas {
            let Some(value) = params.get(&name) else {
                anyhow::ensure!(!spec.required, "missing required parameter {name}");
                continue;
            };
            let values = if let Value::Array(values) = value {
                anyhow::ensure!(
                    spec.repeated && spec.location != "path",
                    "{name} does not accept an array"
                );
                anyhow::ensure!(
                    !spec.required || !values.is_empty(),
                    "{name} cannot be empty"
                );
                values.iter().collect::<Vec<_>>()
            } else {
                vec![value]
            };
            for value in values {
                let text = validate_parameter(&name, &spec, value)?;
                match spec.location.as_str() {
                    "path" => {
                        anyhow::ensure!(!text.is_empty(), "empty path parameter {name}");
                        let reserved = path.contains(&format!("{{+{name}}}"));
                        let encoded = if reserved {
                            text.split('/')
                                .map(escape_segment)
                                .collect::<Result<Vec<_>>>()?
                                .join("/")
                        } else {
                            escape_segment(&text)?
                        };
                        path = path
                            .replace(&format!("{{{name}}}"), &encoded)
                            .replace(&format!("{{+{name}}}"), &encoded);
                    }
                    "query" => query.push((name.clone(), text)),
                    _ => bail!("unsupported parameter location for {name}"),
                }
            }
        }
        anyhow::ensure!(!path.contains(['{', '}']), "unresolved path parameter");
        let base = if let Some(base) = &method.base_url {
            base.trim_end_matches('/').to_owned()
        } else if self.root_url.is_empty() {
            self.base_url.trim_end_matches('/').to_owned()
        } else {
            format!(
                "{}/{}",
                self.root_url.trim_end_matches('/'),
                self.service_path.trim_matches('/')
            )
        };
        let mut url = Url::parse(&format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        ))?;
        contract_url(&url)?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        Ok(url)
    }
}

fn escape_segment(text: &str) -> Result<String> {
    anyhow::ensure!(
        text != "." && text != "..",
        "dot path segments are not allowed"
    );
    Ok(utf8_percent_encode(text, NON_ALPHANUMERIC).to_string())
}

pub(crate) fn validate_parameter(name: &str, spec: &Parameter, value: &Value) -> Result<String> {
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => bail!("{name} must be a scalar value"),
    };
    match spec.kind.as_str() {
        "string" => anyhow::ensure!(value.is_string(), "{name} must be a string"),
        "boolean" => anyhow::ensure!(
            text == "true" || text == "false",
            "{name} must be a boolean"
        ),
        "integer" => {
            let n: i128 = text
                .parse()
                .with_context(|| format!("{name} must be an integer"))?;
            if let Some(min) = &spec.minimum {
                anyhow::ensure!(n >= min.parse::<i128>()?, "{name} is below the minimum");
            }
            if let Some(max) = &spec.maximum {
                anyhow::ensure!(n <= max.parse::<i128>()?, "{name} exceeds the maximum");
            }
        }
        "number" => {
            anyhow::ensure!(
                text.parse::<f64>().is_ok_and(f64::is_finite),
                "{name} must be a finite number"
            );
        }
        "" => {}
        _ => bail!("unsupported parameter type for {name}"),
    }
    anyhow::ensure!(
        spec.choices.is_empty() || spec.choices.contains(&text),
        "invalid value for {name}; expected one of {}",
        spec.choices.join(", ")
    );
    Ok(text)
}

#[cfg(test)]
#[path = "../tests/unit/discovery.rs"]
mod tests;
