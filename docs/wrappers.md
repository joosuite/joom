# Short command wrappers

Run `make build` to create the wrappers in `bin/`. `make install` installs them
beside `joom`. Add that directory to your `PATH`.

`joom` and its wrappers are native executables. Installing and running them
requires no Node.js. Building from source requires Rust and Cargo; `make build`
and `make install` also create the wrapper entry points with a native installer.

```sh
jmmail ACCOUNT to:lara@example.com body:"Hello\nHow are you?"
jmcal ACCOUNT add "Lunch" at:"tomorrow 12:30" for:1h with:lara@example.com
jmdocs ACCOUNT ls limit:10
```

The short commands and `key:value` parameters belong to the wrappers. Existing
`joom` commands keep their syntax. For example, `joom mail ACCOUNT messages.send
--to lara@example.com --text Hello` remains available.

Quote values containing spaces. Repeat parameters to supply several recipients,
attachments or rows. Inline text accepts `\n`, `\t`, and `\\`.
Use `--` before positional text that looks like a named parameter.

Use `file:PATH` to read parameters from a JSON object. File property names use
camelCase, such as `bodyFile` and `matchCase`. Explicit parameters replace file
values, including an entire repeated list. Explicit inline text replaces its
corresponding file input. Required values are checked after combining the inputs.
Select the command definition under `$defs` in
[wrappers.schema.json](../schemas/wrappers.schema.json) to validate complete input.

Lists default to 20 results unless the command describes a date window. Use
`limit:all` to fetch all results. Downloads and exports require `out:PATH` and
never overwrite existing files. CSV imports write literal values, including
text beginning with `=`. Task due dates use the computer's local date. Calendar
dates use the mailbox timezone, or the computer timezone when the mailbox has
none. An explicit `timezone:` value overrides both. Ambiguous times at clock changes
require a timestamp with an explicit offset.

Use `WRAPPER --help` to list its commands, `WRAPPER ACCOUNT COMMAND --help` for
common parameters, and `--help --verbose` for advanced parameters. `json:true`
changes only the output format. Both formats contain the same records and fields,
including message IDs and thread IDs in mail results. Advanced `home:PATH` and
`timeout:SECONDS` select the credential
directory and request timeout.

The wrappers use your existing Joom sign-in. Document, spreadsheet, and presentation
listing use OneDrive. Document sharing and native file exports also use
OneDrive. Authorize it with the service when needed, for example
`joom auth ACCOUNT login docs,drive`. See
[Microsoft support](microsoft.md) for each command group's implemented operations.

Compound commands can perform several requests. If a later request fails, the
error identifies an already-created resource where available. Check that
resource before retrying a creation command.

## jmmail

Read, compose, and organise mail. The default command is `inbox`.

| Command | Description |
| --- | --- |
| `inbox` | Show recent inbox messages. |
| `find` | Search messages. |
| `read` | Read a message and list its attachments. |
| `send` | Compose and send a message. You can omit send when supplying to:, cc:, or bcc:. |
| `draft` | Save a draft. Recipients may be omitted. |
| `drafts` | List saved drafts. |
| `send-draft` | Send an existing draft. |
| `trash` | Move a message to Trash. |
| `mark` | Change a message's read or star state. |

### inbox

Show recent inbox messages.

```text
jmmail ACCOUNT inbox [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `unread:` | no | Show only unread messages. Default: `false`. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### find

Search messages.

```text
jmmail ACCOUNT find QUERY [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `query:` | yes | Outlook Mail search query. |
| `from:` | no | Sender email address. |
| `after:` | no | YYYY-MM-DD, today, or tomorrow. |
| `has:` | no | Require an attachment. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### read

Read a message and list its attachments.

```text
jmmail ACCOUNT read MESSAGE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Message ID. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### send

Compose and send a message. You can omit send when supplying to:, cc:, or bcc:.

```text
jmmail ACCOUNT send [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `to:` | no | Email addresses. Repeat for multiple values. |
| `cc:` | no | Email addresses. Repeat for multiple values. |
| `bcc:` | no | Email addresses. Repeat for multiple values. |
| `subject:` | no | Message subject. |
| `body:` | no | Inline text. Use \n for a new line. |
| `body-file:` | no | Read the message body from a UTF-8 file. |
| `attach:` | no | Local attachment paths. Repeat for multiple values. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

Provide at least one recipient with to:, cc:, or bcc:.

Use body: for inline text or body-file: to read a file.

### draft

Save a draft. Recipients may be omitted.

```text
jmmail ACCOUNT draft [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `to:` | no | Email addresses. Repeat for multiple values. |
| `cc:` | no | Email addresses. Repeat for multiple values. |
| `bcc:` | no | Email addresses. Repeat for multiple values. |
| `subject:` | no | Message subject. |
| `body:` | no | Inline text. Use \n for a new line. |
| `body-file:` | no | Existing local file, up to 1 GiB. |
| `attach:` | no | Local attachment files. Repeat for multiple values. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

Write text with body: or read it from body-file:.

### drafts

List saved drafts.

```text
jmmail ACCOUNT drafts [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### send-draft

Send an existing draft.

```text
jmmail ACCOUNT send-draft DRAFT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### trash

Move a message to Trash.

```text
jmmail ACCOUNT trash MESSAGE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### mark

Change a message's read or star state.

```text
jmmail ACCOUNT mark MESSAGE STATE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `state:` | yes | New message state. Parser choices: `read`, `unread`, `starred`, `unstarred`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

## jmdrive

Find, transfer, and share files. The default command is `ls`.

| Command | Description |
| --- | --- |
| `ls` | List files in a folder. Defaults to the root folder. |
| `find` | Find files by name or content. |
| `info` | Show file metadata, owners, location, and sharing. |
| `get` | Download an existing OneDrive file. |
| `put` | Upload a local file. |
| `mkdir` | Create a folder. |
| `move` | Move or rename a file, preserving its other metadata. |
| `copy` | Copy a file. |
| `share` | Grant someone access to a file. |
| `trash` | Move a file to Trash. |

### ls

List files in a folder. Defaults to the root folder.

```text
jmdrive ACCOUNT ls [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `folder:` | no | File ID or OneDrive URL. Default: `"root"`. |
| `type:` | no | folder, document, spreadsheet, presentation, image, pdf, or a MIME type. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### find

Find files by name or content.

```text
jmdrive ACCOUNT find TEXT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `query:` | yes | Search text. |
| `folder:` | no | File ID or OneDrive URL. |
| `type:` | no | Item type. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### info

Show file metadata, owners, location, and sharing.

```text
jmdrive ACCOUNT info FILE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | File ID or OneDrive URL. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### get

Download an existing OneDrive file.

Downloads the existing OneDrive file.

```text
jmdrive ACCOUNT get FILE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | File ID or OneDrive URL. |
| `out:` | yes | Save to this new file. Existing files are never overwritten. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### put

Upload a local file.

File uploads are limited to 250 MiB. Folder copying is unavailable.

```text
jmdrive ACCOUNT put PATH [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `path:` | yes | Existing local file, up to 250 MiB. |
| `folder:` | no | File ID or OneDrive URL. |
| `name:` | no | Name. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### mkdir

Create a folder.

```text
jmdrive ACCOUNT mkdir NAME [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `name:` | yes | Name. |
| `folder:` | no | File ID or OneDrive URL. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### move

Move or rename a file, preserving its other metadata.

```text
jmdrive ACCOUNT move FILE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | File ID or OneDrive URL. |
| `to:` | no | Destination folder. |
| `name:` | no | Name. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

Provide to: to move the file or name: to rename it.

### copy

Copy a file.

File uploads are limited to 250 MiB. Folder copying is unavailable.

```text
jmdrive ACCOUNT copy FILE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | File ID or OneDrive URL. |
| `to:` | no | File ID or OneDrive URL. |
| `name:` | no | Name. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### share

Grant someone access to a file.

```text
jmdrive ACCOUNT share FILE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | File ID or OneDrive URL. |
| `to:` | yes | Destination or recipient. |
| `role:` | no | Access granted to the recipient. Parser choices: `reader`, `writer`. Default: `"reader"`. |
| `notify:` | no | Send email notifications to affected people. Default: `true`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### trash

Move a file to Trash.

```text
jmdrive ACCOUNT trash FILE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | File ID or OneDrive URL. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

## jmcal

Manage calendar events. The default command is `today`.

| Command | Description |
| --- | --- |
| `today` | Show today's events in calendar time. |
| `week` | Show this week's events, Monday through Sunday. |
| `find` | Find events. Date bounds use calendar time. |
| `show` | Show an event and attendee responses. |
| `add` | Create a timed or all-day event. |
| `move` | Reschedule an event. Its duration is preserved unless for: is supplied. |
| `cancel` | Cancel an event. |
| `calendars` | List available calendars. |

### today

Show today's events in calendar time.

```text
jmcal ACCOUNT today [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `calendar:` | no | Calendar ID. Default: `"primary"`. |
| `timezone:` | no | IANA timezone, such as Europe/Paris. Defaults to the mailbox timezone, or the computer timezone when the mailbox has none. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### week

Show this week's events, Monday through Sunday.

```text
jmcal ACCOUNT week [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `calendar:` | no | Calendar ID. Default: `"primary"`. |
| `timezone:` | no | IANA timezone, such as Europe/Paris. Defaults to the mailbox timezone, or the computer timezone when the mailbox has none. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### find

Find events. Date bounds use calendar time.

```text
jmcal ACCOUNT find TEXT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `query:` | yes | Search text. |
| `from:` | no | YYYY-MM-DD, today, or tomorrow. |
| `until:` | no | YYYY-MM-DD, today, or tomorrow. |
| `calendar:` | no | Calendar ID. Default: `"primary"`. |
| `timezone:` | no | IANA timezone, such as Europe/Paris. Defaults to the mailbox timezone, or the computer timezone when the mailbox has none. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### show

Show an event and attendee responses.

```text
jmcal ACCOUNT show EVENT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `calendar:` | no | Calendar ID. Default: `"primary"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### add

Create a timed or all-day event.

```text
jmcal ACCOUNT add TITLE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `title:` | yes | Title. |
| `at:` | no | RFC 3339, YYYY-MM-DD HH:MM, or tomorrow HH:MM. |
| `on:` | no | YYYY-MM-DD, today, or tomorrow. |
| `for:` | no | Timed event duration, such as 30m or 1h. Maximum 366 days. Default: `"30m"`. |
| `with:` | no | Email addresses. Repeat for multiple values. |
| `where:` | no | Event location or meeting URL. |
| `calendar:` | no | Calendar ID. Default: `"primary"`. |
| `timezone:` | no | IANA timezone, such as Europe/Paris. Defaults to the mailbox timezone, or the computer timezone when the mailbox has none. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

Set a time with at: or an all-day date with on:.

for: sets a timed event's duration; omit it for all-day events.

### move

Reschedule an event. Its duration is preserved unless for: is supplied.

```text
jmcal ACCOUNT move EVENT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `at:` | yes | RFC 3339, YYYY-MM-DD HH:MM, or tomorrow HH:MM. |
| `for:` | no | Positive duration, such as 30m or 1h, up to 366 days. |
| `calendar:` | no | Calendar ID. Default: `"primary"`. |
| `timezone:` | no | IANA timezone, such as Europe/Paris. Defaults to the mailbox timezone, or the computer timezone when the mailbox has none. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### cancel

Cancel an event.

```text
jmcal ACCOUNT cancel EVENT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `calendar:` | no | Calendar ID. Default: `"primary"`. |
| `notify:` | no | Send email notifications to affected people. Parser choices: `True`. Default: `true`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### calendars

List available calendars.

```text
jmcal ACCOUNT calendars [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

## jmsheets

Read and change spreadsheet cells. The default command is `ls`.

| Command | Description |
| --- | --- |
| `ls` | Find spreadsheets. Requires Drive access. |
| `read` | Display cells. Defaults to the first tab, A1:Z20. |
| `set` | Set one cell to a literal value. |
| `load` | Write a CSV file starting at a cell. Values are literal. |
| `clear` | Clear an explicit cell range. |
| `export` | Export one tab as CSV or the workbook in another format. |

### ls

Find spreadsheets. Requires Drive access.

```text
jmsheets ACCOUNT ls [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `name:` | no | Name. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### read

Display cells. Defaults to the first tab, A1:Z20.

```text
jmsheets ACCOUNT read SHEET [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Spreadsheet ID or URL. |
| `tab:` | no | Tab name. |
| `range:` | no | A1 cell range, such as A1:D20. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### set

Set one cell to a literal value.

```text
jmsheets ACCOUNT set SHEET [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Spreadsheet ID or URL. |
| `tab:` | no | Tab name. |
| `cell:` | yes | One A1 cell, such as B2. |
| `value:` | yes | Literal cell value; formulas are not evaluated. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### load

Write a CSV file starting at a cell. Values are literal.

```text
jmsheets ACCOUNT load SHEET [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Spreadsheet ID or URL. |
| `tab:` | no | Tab name. |
| `csv:` | yes | Existing local file, up to 1 GiB. |
| `at:` | no | At. Default: `"A1"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### clear

Clear an explicit cell range.

```text
jmsheets ACCOUNT clear SHEET [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Spreadsheet ID or URL. |
| `tab:` | no | Tab name. |
| `range:` | yes | A1 cell range, such as A1:D20. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### export

Export one tab as CSV or the workbook in another format.

Microsoft export formats are CSV and PDF.

```text
jmsheets ACCOUNT export SHEET [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Spreadsheet ID or URL. |
| `tab:` | no | Tab name. |
| `format:` | no | Format. Parser choices: `csv`, `pdf`. Default: `"csv"`. |
| `out:` | yes | Save to this new file. Existing files are never overwritten. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

tab: selects a CSV export. Other formats export the workbook.

## jmdocs

List, share, and export existing Word files through OneDrive. The default command is `ls`.

| Command | Description |
| --- | --- |
| `ls` | Find documents. Requires Drive access. |
| `export` | Export a document. Requires Drive access. |
| `share` | Grant someone access to a document. Requires Drive access. |

### ls

Find documents. Requires Drive access.

```text
jmdocs ACCOUNT ls [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `name:` | no | Name. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### export

Export a document. Requires Drive access.

Microsoft export supports PDF only.

```text
jmdocs ACCOUNT export DOCUMENT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Document ID or Word URL. |
| `format:` | no | Format. Parser choices: `pdf`. Default: `"pdf"`. |
| `out:` | yes | Save to this new file. Existing files are never overwritten. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### share

Grant someone access to a document. Requires Drive access.

```text
jmdocs ACCOUNT share DOCUMENT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Document ID or Word URL. |
| `to:` | yes | Destination or recipient. |
| `role:` | no | Access granted to the recipient. Parser choices: `reader`, `writer`. Default: `"reader"`. |
| `notify:` | no | Send email notifications to affected people. Default: `true`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

## jmslides

List and export existing PowerPoint files through OneDrive. The default command is `ls`.

| Command | Description |
| --- | --- |
| `ls` | Find presentations. Requires Drive access. |
| `export` | Export a presentation. Requires Drive access. |

### ls

Find presentations. Requires Drive access.

```text
jmslides ACCOUNT ls [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `name:` | no | Name. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### export

Export a presentation. Requires Drive access.

Microsoft export supports PDF only.

```text
jmslides ACCOUNT export PRESENTATION [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Presentation ID or URL. |
| `format:` | no | Format. Parser choices: `pdf`. Default: `"pdf"`. |
| `out:` | yes | Save to this new file. Existing files are never overwritten. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

## jmtasks

Manage tasks and task lists. The default command is `ls`.

| Command | Description |
| --- | --- |
| `ls` | Show incomplete tasks in a list. |
| `add` | Create a task. |
| `show` | Show task details. |
| `done` | Mark a task complete. |
| `undo` | Mark a task incomplete. |
| `set` | Change selected task fields. |
| `delete` | Delete a task. |
| `lists` | List task lists. |
| `new-list` | Create a task list. |

### ls

Show incomplete tasks in a list.

```text
jmtasks ACCOUNT ls [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `list:` | no | Task list ID. Default: `"@default"`. |
| `due:` | no | YYYY-MM-DD, today, or tomorrow. |
| `completed:` | no | Include completed tasks. Default: `false`. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### add

Create a task.

```text
jmtasks ACCOUNT add TITLE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `title:` | yes | Title. |
| `due:` | no | YYYY-MM-DD, today, or tomorrow. |
| `notes:` | no | Inline text. Use \n for a new line. |
| `list:` | no | Task list ID. Default: `"@default"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### show

Show task details.

```text
jmtasks ACCOUNT show TASK [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `list:` | no | Task list ID. Default: `"@default"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### done

Mark a task complete.

```text
jmtasks ACCOUNT done TASK [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `list:` | no | Task list ID. Default: `"@default"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### undo

Mark a task incomplete.

```text
jmtasks ACCOUNT undo TASK [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `list:` | no | Task list ID. Default: `"@default"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### set

Change selected task fields.

```text
jmtasks ACCOUNT set TASK [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `title:` | no | Title. |
| `due:` | no | YYYY-MM-DD, today, or tomorrow. |
| `notes:` | no | Inline text. Use \n for a new line. |
| `list:` | no | Task list ID. Default: `"@default"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

Provide title:, due:, or notes: to change the task.

### delete

Delete a task.

```text
jmtasks ACCOUNT delete TASK [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `list:` | no | Task list ID. Default: `"@default"`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### lists

List task lists.

```text
jmtasks ACCOUNT lists [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### new-list

Create a task list.

```text
jmtasks ACCOUNT new-list TITLE [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `title:` | yes | Title. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

## jmcontacts

Find and maintain contacts. The default command is `ls`.

| Command | Description |
| --- | --- |
| `ls` | List contacts. |
| `find` | Find contacts by name, email, or phone. |
| `show` | Show contact details. |
| `new` | Create a contact. |
| `set` | Change named fields and preserve all others. Repeated email: and phone: values replace those lists. |
| `delete` | Delete a contact. |

### ls

List contacts.

```text
jmcontacts ACCOUNT ls [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### find

Find contacts by name, email, or phone.

```text
jmcontacts ACCOUNT find TEXT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `query:` | yes | Search text. |
| `limit:` | no | Maximum results, or all. Default: `20`. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### show

Show contact details.

```text
jmcontacts ACCOUNT show CONTACT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### new

Create a contact.

```text
jmcontacts ACCOUNT new NAME [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `name:` | yes | Name. |
| `email:` | no | Email addresses. Repeat for multiple values. |
| `phone:` | no | Phone numbers. Repeat for multiple values. |
| `company:` | no | Company name. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

### set

Change named fields and preserve all others. Repeated email: and phone: values replace those lists.

```text
jmcontacts ACCOUNT set CONTACT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `name:` | no | Name. |
| `email:` | no | Email addresses. Repeat for multiple values. |
| `phone:` | no | Phone numbers. Repeat for multiple values. |
| `company:` | no | Company name. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

Provide name:, email:, phone:, or company: to change the contact.

### delete

Delete a contact.

```text
jmcontacts ACCOUNT delete CONTACT [key:value ...]
```

| Parameter | Required | Description |
| --- | --- | --- |
| `target:` | yes | Resource ID shown by its list command. |
| `home:` | no | Store credentials and cache in this directory. Advanced parameter. |
| `timeout:` | no | Request timeout in seconds. Default: `30`. Allowed range: 1 to 600. Advanced parameter. |
| `json:` | no | Print the same result data as JSON. Default: `false`. |

