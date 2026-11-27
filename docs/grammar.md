# Command reference

For short commands such as `jmmail ACCOUNT to:lara@example.com body:Hello`,
see [Short command wrappers](wrappers.md). The wrappers have their own commands
and parameter syntax. The direct CLI exposes implemented Microsoft operations; see
[Microsoft support](microsoft.md) for provider limits.

## Usage

```text
joom [OPTIONS] SERVICE ACCOUNT COMMAND [ARGUMENTS] [COMMAND_OPTIONS]
joom [OPTIONS] auth [ACCOUNT] COMMAND
```

`make build` builds the current source and updates `./bin/joom`. Run its help
directly from the workspace:

```sh
./bin/joom mail ACCOUNT messages.send --help
```

`make install` updates the installed `joom` executable.

`joom` and its wrappers are native Rust executables. The build and install steps
use Rust and Cargo. TypeScript maintainer tools use Node.js 22 or newer, with
dependencies installed by `npm ci`.

| Option | Description |
| --- | --- |
| `--home PATH` | Store new state in `PATH/joom`. Existing `PATH/joom-rust` vaults stay in place. |
| `--timeout SECONDS` | Request timeout, 1 through 600 seconds; default 30. |
| `--json` | Output the translated compatibility response as JSON. |
| `--version` | Print the installed version. |
| `--help --verbose` | Include advanced parameters in command help. |

Results use plain text by default. Read commands display records or text; writes
confirm the operation and show the resource ID when Microsoft returns one. List
output shows IDs and useful fields instead of transport metadata. `--fields`
prints your selected response fields; `--json` returns the translated
compatibility response.

## Authentication

```text
joom auth init [--headless]
joom auth enroll [--stdin]
joom auth test
joom auth list
joom auth ACCOUNT login SERVICE_LIST [--personal]
joom auth ACCOUNT all [--personal]
joom auth [ACCOUNT] export FILE
joom auth [ACCOUNT] import FILE [--headless] [--replace]
joom auth ACCOUNT logout
```

`init` creates or reuses the OS-keychain master key; `--headless` selects a private
local key file for a new vault. `enroll` saves your Microsoft Application client
ID, with masked input.
`--stdin` accepts a JSON object containing `client_id`. Native PKCE sign-in does
not send a client secret. `test` checks Microsoft OAuth connectivity without
verifying registration or sign-in. `--personal` selects the consumer endpoint.

`list` prints registered account emails, one per line. Use `joom --json auth list`
for a JSON array. It reads the selected vault's account registrations without
unlocking credentials or contacting Microsoft. An empty vault produces no text,
or `[]` in JSON.

`login` opens your browser and authorizes the comma-separated services, for
example `mail,drive,calendar`. Use `login all` to request every supported service's
configured scopes. The selected Microsoft account must match `ACCOUNT`.
`auth ACCOUNT all` is the shortcut for `auth ACCOUNT login all`.
`mail` authorizes Outlook Mail. Include all services you want to use
when signing in again. `logout` deletes that account's local tokens; revoke
Microsoft permissions separately in your Microsoft account settings.

See [sign-in setup](authentication.md).

For a VPS without a browser, export credentials on your laptop with
`joom auth ACCOUNT export credentials.joom`, copy the encrypted file with `scp`,
then run `joom auth import credentials.joom --headless` on the VPS. Both commands
prompt for the transfer passphrase. Export includes the OAuth client and refresh
tokens; import creates the server vault without a desktop keychain. Use
`--replace` to replace existing account credentials. Advanced
`--passphrase-file PATH` reads a private passphrase file for automation.

## Commands

### mail

Outlook folders replace label-management operations. Thread operations and raw
MIME retrieval are unavailable. Message label mutations support UNREAD and STARRED.

```text
joom mail ACCOUNT read MESSAGE_ID
joom mail ACCOUNT unread MESSAGE_ID
joom mail ACCOUNT star MESSAGE_ID
joom mail ACCOUNT unstar MESSAGE_ID
```

```sh
joom mail ACCOUNT profile
joom mail ACCOUNT labels.list
joom mail ACCOUNT messages.list --q is:unread
joom mail ACCOUNT messages.get --id MESSAGE_ID
joom mail ACCOUNT messages.send --to alice@example.com --cc bob@example.com --bcc archive@example.com --subject 'Hello' --text 'Hi Alice'
joom mail ACCOUNT messages.send --file examples/requests/mail-compose.json
joom mail ACCOUNT drafts.create --to alice@example.com --subject 'Draft' --text-file draft.txt
```

[Full command help](services/mail.md).

`messages.send` and `drafts.create` accept `--to`, `--cc`,
`--bcc`, `--from`, `--reply-to`, `--subject`, `--text`, `--html`, `--text-file`,
`--html-file`, and `--attach`. Repeat recipient and attachment options for
multiple values. Addresses use bare ASCII mailboxes, such as
`alice@example.com`. `--from` defaults to `ACCOUNT`; Microsoft must authorize that
sender. Text and HTML together create a multipart message. Attachments use
`application/octet-stream`.

Body files must contain UTF-8 text. File paths resolve from the working directory.
Composition inputs have a combined 16 MiB content limit, and the final encoded
JSON request must also fit within 16 MiB. Mail media uploads are unavailable.
Composition cannot be combined with `--upload` or raw message content.
Sending requires at least one recipient in `to`, `cc`, or `bcc`. Drafts can omit
recipients. An omitted body becomes empty plain text.

`drafts.send` requires the draft ID. With no composition inputs it sends the
existing content. Replacing draft content at send time is not implemented.

### drive

OneDrive queries and field mappings are a subset of Drive inputs. `files.delete`
recycles an item. File uploads are limited to 250 MiB.

```text
joom drive ACCOUNT search TEXT [--parent ID] [--type MIME]
```

```sh
joom drive ACCOUNT files.list
joom drive ACCOUNT files.get --file-id FILE_ID
joom drive ACCOUNT files.export --file-id FILE_ID --mime-type application/pdf --output report.pdf
joom drive ACCOUNT permissions.create --file-id FILE_ID --type user --role reader --email-address alice@example.com
```

[Full command help](services/drive.md).

### calendar

Outlook sends meeting updates automatically. Notification suppression and
RFC5545 recurrence writes are unavailable.

```text
joom calendar ACCOUNT events [--calendar ID] [--from TIMESTAMP] [--until TIMESTAMP] [--query TEXT]
```

```sh
joom calendar ACCOUNT calendars.list
joom calendar ACCOUNT events.get --event-id EVENT_ID
joom calendar ACCOUNT events.insert --file event.json
```

[Full command help](services/calendar.md).

`events` expands recurring events and orders them by start time. The default window starts now and ends 30 days later. `--from` and `--until` select a different interval. `--calendar` defaults to `primary`.

### sheets

Microsoft mappings support worksheet metadata and range get, update, and clear.
Workbook creation, append, and batch operations are unavailable.

```sh
joom sheets ACCOUNT get --spreadsheet-id SPREADSHEET_ID
joom sheets ACCOUNT values.get --spreadsheet-id SPREADSHEET_ID --range 'Sheet1!A1:D10'
joom sheets ACCOUNT values.update --file values.json
```

[Full command help](services/sheets.md).

### tasks

Microsoft To Do supports list and task operations. Clear, move, hierarchy, and
position inputs are unavailable.

```sh
joom tasks ACCOUNT tasklists.list
joom tasks ACCOUNT tasks.list
joom tasks ACCOUNT tasks.insert --title TITLE
```

[Full command help](services/tasks.md).

### contacts

These commands operate on Outlook contacts. Directory, group, other-contact,
batch, and photo operations are unavailable.

```sh
joom contacts ACCOUNT people.get --resource-name people/CONTACT_ID
joom contacts ACCOUNT people.connections.list --resource-name people/me
joom contacts ACCOUNT people.create-contact --names '[{"givenName":"Alice"}]'
```

[Full command help](services/contacts.md).

Word and PowerPoint files use the `jmdocs` and `jmslides` wrappers for listing
and PDF export. `jmdocs` also shares files. See [wrappers](wrappers.md).

## Request and output options

| Option | Description |
| --- | --- |
| `--file PATH` | Read request parameters and a body from a JSON file. Named options override file values. |
| `--body JSON` | Supply the request body inline. |
| `--fields MASK` | Select response fields. |
| `--max N` | Limit the total number of results. The default fetches all results. |
| `--max-pages N` | Stop after this many pages. |
| `--ttl MINUTES` | Reuse encrypted read results up to this age. The default, 0, fetches fresh data. |
| `--output PATH`, `-o PATH` | Save a download to a new file. Existing files are preserved. |
| `--upload PATH` | Upload a local file when the command supports media uploads. |
| `--content-type MIME` | Set the upload MIME type. |

Command help lists the available options. Complex fields accept JSON. Repeated
fields accept multiple values or repeated options. Boolean options support
`--option` and `--option=false`.

Request files describe CLI parameters with camelCase property names, such as
`fileId`, `startTime`, and `text`. See [schemas and examples](file-schemas.md).
Commands validate requests before accessing account credentials.

Read commands fetch all pages unless `--max` or `--max-pages` stops them.
Page sizes in request files control individual requests. Paginated JSON output
contains one translated response per line.

`--ttl 60` measures freshness from the original fetch time. Cached account data
stays encrypted. Writes and login changes invalidate its cache. Downloads and
ID or token generation always fetch fresh data. A cached default Calendar window
retains its original start and end until its TTL expires.

## Services

| Service | Microsoft service |
| --- | --- |
| `mail` | Outlook Mail |
| `drive` | OneDrive |
| `calendar` | Outlook Calendar |
| `sheets` | Excel |
| `tasks` | Microsoft To Do |
| `contacts` | Outlook Contacts |

## Help

`--help` lists common parameters. Add `--verbose` to include advanced parameters
and full descriptions. Both views group mandatory and optional parameters.
The `Input choices` section explains alternatives, such as using a date for an
all-day event or a date and time for a timed event.

Mandatory values can come from `--file` or command-line options. `--verbose` is
only available with help.

```sh
joom --help
joom help mail
joom mail ACCOUNT messages.send --help
joom mail ACCOUNT messages.send --help --verbose
joom help sheets values.update
joom help contacts people.get
```
