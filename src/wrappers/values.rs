use anyhow::{Context, Result, bail, ensure};
use jiff::{
    SignedDuration, Timestamp, ZonedDateTime,
    civil::{Date, Time},
    tz::TimeZone,
};
use serde_json::{Map, Value, json};
use std::{io::Write, path::Path};

pub fn unescape(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

pub fn date(text: &str, today: Date) -> Result<Date> {
    match text {
        "today" => Ok(today),
        "tomorrow" => today
            .tomorrow()
            .context("date is outside its allowed range"),
        _ => {
            ensure!(text.len() == 10, "use YYYY-MM-DD, today, or tomorrow");
            text.parse::<Date>()
                .context("use YYYY-MM-DD, today, or tomorrow")
        }
    }
}

pub fn duration(text: &str) -> Result<SignedDuration> {
    let unit = text
        .chars()
        .last()
        .context("use a duration such as 30m or 1h")?;
    let number = text.strip_suffix(unit).unwrap();
    let number = number
        .parse::<i64>()
        .context("use a duration such as 30m or 1h")?;
    let seconds = number
        .checked_mul(match unit {
            'm' => 60,
            'h' => 3600,
            'd' => 86400,
            _ => bail!("use m, h, or d for duration units"),
        })
        .context("duration is too large")?;
    let maximum = super::catalog()["$defs"]["duration"]["x-max-seconds"]
        .as_i64()
        .expect("duration constraint");
    ensure!(
        (1..=maximum).contains(&seconds),
        "duration must be positive and at most 366 days"
    );
    Ok(SignedDuration::from_secs(seconds))
}

pub fn timestamp(text: &str, zone: &TimeZone, now: Timestamp) -> Result<Timestamp> {
    if let Ok(time) = text.parse::<Timestamp>() {
        return Ok(time);
    }
    let (day, time) = text
        .rsplit_once(' ')
        .or_else(|| text.split_once('T'))
        .context("use YYYY-MM-DD HH:MM, tomorrow HH:MM, or an RFC 3339 timestamp")?;
    let day = date(day, now.to_zoned_datetime(zone.clone()).date())?;
    let time = Time::strptime("%H:%M", time).context("use HH:MM in 24-hour time")?;
    zone.to_ambiguous_zoned_datetime(day.to_datetime(time)).unambiguous().map(|time| time.timestamp()).context("this local time is missing or ambiguous at a clock change; provide an RFC 3339 timestamp with an explicit offset")
}

pub fn midnight(day: Date, zone: &TimeZone) -> Result<Timestamp> {
    day.to_zoned_datetime(zone.clone())
        .map(|time| time.timestamp())
        .context("midnight is unavailable on this date; use a different calendar timezone")
}

pub fn rfc3339(time: Timestamp, zone: &TimeZone) -> String {
    time.to_zoned_datetime(zone.clone())
        .strftime("%Y-%m-%dT%H:%M:%S%:z")
        .to_string()
}

pub fn format(kind: &str, value: &str) -> Result<()> {
    match kind {
        "single-line" => ensure!(
            !value.chars().any(char::is_control),
            "use a single line without control characters"
        ),
        "email" => {
            ensure!(
                value.is_ascii()
                    && !value
                        .chars()
                        .any(|ch| ch.is_control() || ch.is_whitespace() || "<>,;\"".contains(ch)),
                "use a bare email address"
            );
            let (local, domain) = value
                .split_once('@')
                .context("use an email address containing @")?;
            ensure!(
                !local.is_empty() && !domain.is_empty() && !domain.contains('@'),
                "use a valid email address"
            );
        }
        "local-date" => {
            date(value, ZonedDateTime::now().date())?;
        }
        "local-time" => {
            if value.parse::<Timestamp>().is_err() {
                let (day, time) = value
                    .rsplit_once(' ')
                    .or_else(|| value.split_once('T'))
                    .context("use YYYY-MM-DD HH:MM or tomorrow HH:MM")?;
                date(day, ZonedDateTime::now().date())?;
                Time::strptime("%H:%M", time).context("use HH:MM in 24-hour time")?;
            }
        }
        "date-time" => {
            value
                .parse::<Timestamp>()
                .context("use an RFC 3339 timestamp with a timezone")?;
        }
        "duration" => {
            duration(value)?;
        }
        "iana-time-zone" => {
            TimeZone::get(value).context("unknown IANA timezone")?;
        }
        "local-file" => {
            let metadata = std::fs::metadata(value).context("cannot read local file")?;
            let maximum = super::catalog()["$defs"]["path"]["x-max-bytes"]
                .as_u64()
                .expect("file constraint");
            ensure!(
                metadata.is_file() && metadata.len() <= maximum,
                "use a regular file up to 1 GiB"
            );
        }
        "new-file" => {
            ensure!(!Path::new(value).exists(), "output path already exists");
            let parent = Path::new(value)
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            ensure!(parent.is_dir(), "output directory does not exist");
        }
        "single-cell" | "cell-range" => {
            let cell = value.rsplit('!').next().unwrap();
            let endpoints = cell.split(':').collect::<Vec<_>>();
            ensure!(
                (1..=2).contains(&endpoints.len())
                    && (kind != "single-cell" || endpoints.len() == 1),
                "use an A1 cell or range"
            );
            for endpoint in endpoints {
                let endpoint = endpoint.strip_prefix('$').unwrap_or(endpoint);
                let column_end = endpoint.bytes().take_while(u8::is_ascii_alphabetic).count();
                let (column, row) = endpoint.split_at(column_end);
                let row = row.strip_prefix('$').unwrap_or(row);
                ensure!(
                    !column.is_empty() || !row.is_empty(),
                    "use an A1 cell or range"
                );
                ensure!(
                    column.len() <= 3
                        && (row.is_empty() || row.parse::<u64>().is_ok_and(|row| row > 0)),
                    "use an A1 cell or range with positive row numbers"
                );
                if kind == "single-cell" {
                    ensure!(
                        !column.is_empty() && !row.is_empty(),
                        "use one A1 cell, such as B2"
                    );
                }
            }
        }
        _ => bail!("unsupported input format {kind}"),
    }
    Ok(())
}

pub fn resource(kind: &str, value: &str) -> Result<String> {
    if !value.contains("://") {
        let id = value.strip_prefix("people/").unwrap_or(value);
        ensure!(
            !id.is_empty()
                && id.len() <= 2048
                && id != "."
                && id != ".."
                && !id
                    .chars()
                    .any(|c| c.is_control() || c.is_whitespace() || "?#\\".contains(c)),
            "use a Microsoft resource ID or a supported OneDrive URL"
        );
        return Ok(if kind == "contacts" {
            format!("people/{id}")
        } else {
            value.to_owned()
        });
    }
    let url = url::Url::parse(value).context("invalid resource URL")?;
    ensure!(
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.port_or_known_default() == Some(443)
            && url.fragment().is_none(),
        "use an HTTPS Microsoft resource URL"
    );
    let id = if url.host_str() == Some("onedrive.live.com") {
        url.query_pairs()
            .find(|(key, _)| key == "resid" || key == "id")
            .map(|(_, value)| value.into_owned())
    } else if url.host_str() == Some("graph.microsoft.com")
        && url.path().starts_with("/v1.0/me/drive/items/")
    {
        url.path_segments()
            .and_then(|mut parts| parts.nth(4))
            .map(|id| {
                percent_encoding::percent_decode_str(id)
                    .decode_utf8_lossy()
                    .into_owned()
            })
    } else {
        None
    }
    .context(
        "this URL does not contain a supported Microsoft resource ID; use the Graph item ID",
    )?;
    resource(kind, &id)
}

pub fn normalize(shape: &Value, value: &mut Value) -> Result<()> {
    if let Some(items) = value.as_array_mut() {
        for item in items {
            normalize(&shape["items"], item)?;
        }
    } else if let Some(kind) = shape["x-resource"].as_str() {
        *value = json!(resource(
            kind,
            value.as_str().context("resource must be a string")?
        )?);
    }
    Ok(())
}

pub fn read_text(path: &str) -> Result<String> {
    String::from_utf8(crate::http::limited(
        std::fs::File::open(path).context("cannot read text file")?,
        crate::http::DISCOVERY_LIMIT,
    )?)
    .context("text files must contain UTF-8")
}

pub fn text(params: &Map<String, Value>, field: &str) -> Result<Option<String>> {
    if let Some(path) = params.get(&format!("{field}File")).and_then(Value::as_str) {
        Ok(Some(read_text(path)?))
    } else {
        Ok(params.get(field).and_then(Value::as_str).map(str::to_owned))
    }
}

pub fn csv_rows(params: &Map<String, Value>) -> Result<Vec<Vec<String>>> {
    let text = if let Some(path) = params.get("csv").and_then(Value::as_str) {
        read_text(path)?
    } else {
        params
            .get("row")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    };
    let rows = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(text.as_bytes())
        .records()
        .map(|row| row.map(|row| row.iter().map(str::to_owned).collect::<Vec<_>>()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("CSV rows must have the same number of fields and valid quoting")?;
    ensure!(!rows.is_empty(), "CSV input contains no rows");
    Ok(rows)
}

pub fn preflight(params: &Map<String, Value>, calendar_dates: bool) -> Result<()> {
    for name in ["bodyFile", "textFile"] {
        if let Some(path) = params.get(name).and_then(Value::as_str) {
            read_text(path)?;
        }
    }
    for path in params
        .get("attachments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        format("local-file", path)?;
    }
    if params.contains_key("csv") || params.contains_key("row") {
        csv_rows(params)?;
    }
    if !calendar_dates
        && let (Some(from), Some(until)) = (
            params.get("from").and_then(Value::as_str),
            params.get("until").and_then(Value::as_str),
        )
    {
        ensure!(
            date(from, ZonedDateTime::now().date())? <= date(until, ZonedDateTime::now().date())?,
            "until: must be on or after from:"
        );
    }
    Ok(())
}

pub fn save(bytes: &[u8], path: &str) -> Result<Value> {
    let parent = Path::new(path)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|error| error.error)?;
    Ok(json!({"path":path,"bytes":bytes.len()}))
}

pub fn csv_bytes(rows: &[Vec<String>]) -> Result<Vec<u8>> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    for row in rows {
        writer.write_record(
            row.iter()
                .map(String::as_str)
                .chain(std::iter::repeat_n("", width - row.len())),
        )?;
    }
    writer
        .into_inner()
        .map_err(|error| error.into_error().into())
}

pub fn mime(format: &str) -> Result<&'static str> {
    Ok(match format {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "csv" => "text/csv",
        "txt" => "text/plain",
        "html" => "text/html",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        _ => bail!("unsupported export format"),
    })
}

pub fn upload_mime(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    }
}
