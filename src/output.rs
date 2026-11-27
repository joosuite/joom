use anyhow::Result;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    io::{self, Write},
};

pub fn emit(value: &Value, json: bool, compact: bool) -> Result<()> {
    emit_to(value, json, compact, &mut io::stdout().lock())
}

pub fn status(message: &str, value: &Value, json: bool) -> Result<()> {
    status_to(message, value, json, &mut io::stdout().lock())
}

fn status_to(message: &str, value: &Value, json: bool, out: &mut impl Write) -> Result<()> {
    if json {
        emit_to(value, true, false, out)
    } else {
        writeln!(out, "{}", escaped(&Value::String(message.into())))?;
        Ok(())
    }
}

fn emit_to(value: &Value, json: bool, compact: bool, out: &mut impl Write) -> Result<()> {
    if !json {
        plain(value, out, 0)?;
    } else {
        if compact {
            serde_json::to_writer(&mut *out, value)?;
        } else {
            serde_json::to_writer_pretty(&mut *out, value)?;
        }
        writeln!(out)?;
    }
    Ok(())
}

fn escaped(value: &Value) -> String {
    match value {
        Value::String(text) => escaped_text(text),
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(escaped).collect::<Vec<_>>().join(", "),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| format!("{}: {}", escaped_text(key), escaped(value)))
            .collect::<Vec<_>>()
            .join("; "),
    }
}

fn escaped_text(raw: &str) -> String {
    raw.chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            c if c.is_control()
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') =>
            {
                format!("\\u{:04x}", c as u32).chars().collect()
            }
            c => vec![c],
        })
        .collect()
}

fn plain(value: &Value, out: &mut impl Write, indent: usize) -> Result<()> {
    let array = value.as_array().or_else(|| {
        let object = value.as_object()?;
        let mut data = object.iter().filter(|(key, _)| {
            !matches!(
                key.as_str(),
                "kind" | "etag" | "nextPageToken" | "nextSyncToken" | "resultSizeEstimate"
            )
        });
        let (_, collection) = data.next()?;
        if data.next().is_none() {
            collection.as_array()
        } else {
            None
        }
    });
    let padding = " ".repeat(indent);
    if let Some(rows) = array {
        if rows.iter().all(Value::is_object) {
            let columns = rows
                .iter()
                .filter_map(Value::as_object)
                .flat_map(|row| row.keys())
                .collect::<BTreeSet<_>>();
            let mut columns = columns.into_iter().collect::<Vec<_>>();
            columns.sort_by_key(|name| match name.as_str() {
                "id" => 0,
                "name" => 1,
                "title" => 2,
                "type" => 3,
                _ => 4,
            });
            if !columns.is_empty() {
                writeln!(
                    out,
                    "{padding}{}",
                    columns
                        .iter()
                        .map(|name| escaped_text(name))
                        .collect::<Vec<_>>()
                        .join("\t")
                )?;
            }
            for row in rows {
                let row = row.as_object().expect("object rows checked above");
                writeln!(
                    out,
                    "{padding}{}",
                    columns
                        .iter()
                        .map(|key| escaped(row.get(*key).unwrap_or(&Value::Null)))
                        .collect::<Vec<_>>()
                        .join("\t")
                )?;
            }
        } else {
            for row in rows {
                if let Some(cells) = row.as_array() {
                    writeln!(
                        out,
                        "{padding}{}",
                        cells.iter().map(escaped).collect::<Vec<_>>().join("\t")
                    )?;
                } else {
                    plain(row, out, indent)?;
                }
            }
        }
    } else if let Some(object) = value.as_object() {
        for (key, value) in object {
            let key = escaped_text(key);
            if key == "text" && value.as_str().is_some_and(|text| text.contains('\n')) {
                writeln!(out, "{padding}{key}:")?;
                for line in value.as_str().unwrap().lines() {
                    writeln!(out, "{padding}  {}", escaped_text(line))?;
                }
            } else if value.is_array() || value.is_object() {
                writeln!(out, "{padding}{key}:")?;
                plain(value, out, indent + 2)?;
            } else {
                writeln!(out, "{padding}{key}\t{}", escaped(value))?;
            }
        }
    } else {
        writeln!(out, "{padding}{}", escaped(value))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/output.rs"]
mod tests;
