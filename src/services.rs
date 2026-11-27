use anyhow::Result;
use serde_json::{Map, Value};

pub struct Service {
    pub cli: &'static str,
    pub api: &'static str,
    pub version: &'static str,
    pub title: &'static str,
    pub scope: &'static str,
}

macro_rules! services {
    ($($cli:literal, $api:literal, $version:literal, $title:literal, $scope:literal;)*) => {
        pub const SERVICES: &[Service] = &[$(Service { cli: $cli, api: $api, version: $version, title: $title, scope: $scope }),*];
    };
}

services! {
    "mail", "mail", "v1.0", "Outlook Mail", "Mail.ReadWrite Mail.Send";
    "drive", "drive", "v1.0", "OneDrive", "Files.ReadWrite";
    "calendar", "calendar", "v1.0", "Outlook Calendar", "Calendars.ReadWrite MailboxSettings.Read";
    "sheets", "sheets", "v1.0", "Excel", "Files.ReadWrite";
    "tasks", "tasks", "v1.0", "Microsoft To Do", "Tasks.ReadWrite";
    "contacts", "contacts", "v1.0", "Outlook Contacts", "Contacts.ReadWrite";
}

pub fn scopes(services: &str) -> Result<Vec<String>> {
    let mut scopes = Vec::new();
    for name in services.split(',').map(str::trim) {
        let name = match name {
            "docs" | "slides" => "drive",
            _ => name,
        };
        let selected = if name == "all" {
            SERVICES.iter().map(|s| s.cli).collect::<Vec<_>>()
        } else {
            anyhow::ensure!(
                SERVICES.iter().any(|s| s.cli == name),
                "unsupported OAuth service {name}"
            );
            vec![name]
        };
        for name in selected {
            let service = SERVICES
                .iter()
                .find(|s| s.cli == name)
                .expect("known service");
            let permissions = service.scope.split_whitespace();
            for permission in permissions {
                if !scopes.iter().any(|s| s == permission) {
                    scopes.push(permission.to_owned());
                }
            }
        }
    }
    anyhow::ensure!(
        !scopes.is_empty(),
        "the selected service has no Microsoft Graph equivalent; see docs/microsoft.md"
    );
    Ok(scopes)
}

pub struct Invocation {
    pub options: Option<clap::ArgMatches>,
    pub method: String,
    pub params: Map<String, Value>,
    pub body: Option<Value>,
    pub custom_fields: bool,
    pub relative_times: Vec<&'static str>,
}

#[cfg(test)]
#[path = "../tests/unit/services.rs"]
mod tests;
