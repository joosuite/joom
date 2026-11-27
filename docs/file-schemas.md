# CLI parameter files

The short wrappers, such as `jmmail` and `jmcal`, use the command definitions in
[wrappers.schema.json](../schemas/wrappers.schema.json). See the
[wrapper guide](wrappers.md) for their `file:PATH` syntax and parameters.
The `--file` format below applies to `joom`.

`--file PATH` reads the command's parameters from a JSON object. The property
names are the camelCase forms of the command-line options. File parameters and
named command-line options describe the same inputs.

For example, `rename.json` contains:

```json
{
  "fileId": "FILE_ID",
  "name": "Quarterly report"
}
```

These invocations build the same request:

```sh
joom drive ACCOUNT files.update --file rename.json
joom drive ACCOUNT files.update --file-id FILE_ID --name 'Quarterly report'
```

An explicit command-line value overrides its file value:

```sh
joom drive ACCOUNT files.update --file rename.json --name 'Annual report'
```

A scalar replaces its previous value. An explicit repeated option replaces the
file's entire array; repeated occurrences on the command line form the new array.
Boolean parameters accept `--option` and `--option=false`. File booleans use JSON
booleans. Query parameters also retain their supported numeric and boolean string
forms. Complex parameters accept an object or array in the file and JSON text
on the command line. Nullable input fields accept JSON `null` in files.

A parameter file and the merged request body are each limited to 16 MiB. Paths
resolve from the working directory. `--file` takes an ordinary filesystem path.

## Mail parameters

`messages.send` and `drafts.create` accept composition parameters directly:

```json
{
  "to": ["alice@example.com"],
  "cc": ["bob@example.com"],
  "bcc": ["archive@example.com"],
  "subject": "Quarterly report",
  "text": "The report is ready."
}
```

```sh
joom mail ACCOUNT messages.send --file examples/requests/mail-compose.json
joom mail ACCOUNT messages.send --to alice@example.com --cc bob@example.com --bcc archive@example.com --subject 'Quarterly report' --text 'The report is ready.'
```

The schema also describes `from`, `replyTo`, `html`, `textFile`, `htmlFile`, and
`attach`, an array of attachment paths. Their options are `--from`, `--reply-to`,
`--html`, `--text-file`, `--html-file`, and repeated `--attach`.

Joom creates MIME content and encodes it for Outlook Mail. Sending requires at least one
recipient after file and CLI inputs are merged. Drafts can omit recipients.
Addresses are bare ASCII mailboxes. The sender defaults to `ACCOUNT`; Microsoft
checks whether the account can use an explicit sender. Body files must be UTF-8.
Text and HTML together produce a MIME alternative. Attachments use
`application/octet-stream`. Omitted content becomes empty plain text.

Choose `text` or `textFile`, and `html` or `htmlFile`. An explicit body-format
option replaces the file's other representation of that format. Compose inputs
conflict with raw MIME content and media uploads. `drafts.send` requires its
draft ID and sends the existing content.
Replacing draft content at send time and mail media uploads are not implemented.

## Nested input fields

Objects can be supplied as structured JSON or through their named leaf inputs.
For example, a Calendar event file can contain:

```json
{
  "summary": "Planning",
  "startTime": "2026-12-01T10:00:00+01:00",
  "endTime": "2026-12-01T11:00:00+01:00",
  "attendees": [{"email": "alice@example.com"}]
}
```

```sh
joom calendar ACCOUNT events.insert --summary Planning --start-time '2026-12-01T10:00:00+01:00' --end-time '2026-12-01T11:00:00+01:00' --attendees '[{"email":"alice@example.com"}]'
```

Contact record arrays such as `names`, `emailAddresses`, and `phoneNumbers`
retain their structured types. Each record accepts only the fields listed in the command schema.

The schemas describe implemented Microsoft operations. Unsupported commands
and fields are absent. Every accepted option is listed in its schema and help;
see [Microsoft support](microsoft.md) for provider limits.

## Local options and convenience commands

Command-local controls can also come from the file when that command supports
them. They include `fields`, `max`, `maxPages`, `ttl`, `upload`, `contentType`, and
`output`. They affect Joom's behavior and are not sent as Microsoft request fields.
Explicit options override file values. Global account, authentication, output
mode, home directory, and timeout context remain outside parameter files.

Convenience commands have their own definitions. For example, `drive search`
accepts `query`, `parent`, and `type`. Required positional inputs such as a
message ID or search query can come from the file. Joom validates them after
merging the file and command line.

List commands fetch all pages unless `max` or `maxPages` stops them. `pageSize`
and `maxResults`, where supported, control individual request sizes and now also
have named options. `ttl` measures cache freshness in minutes. Writes invalidate
the account's cached reads. Downloads and generated IDs or tokens fetch fresh
data.

## Schemas and examples

Schemas use JSON Schema Draft 2020-12. Select the definition for the command you
are invoking, such as `schemas/drive.schema.json#/$defs/files.update`. Root
validation accepts any command and does not identify which command to run.
Each definition describes the complete command input, including required values
and choices that cannot be combined. A file may omit values supplied by command-line
options. To validate such a file separately, add those values before validating
against the command definition. Joom checks the combined inputs before executing
the command.

The [request example index](../examples/requests/README.md) lists the command for
each file. List examples use `max` to limit the total number of results.

| Service | CLI parameter schema | Read example | Write example | Command reference |
| --- | --- | --- | --- | --- |
| `mail` | [mail.schema.json](../schemas/mail.schema.json) | [mail.json](../examples/requests/mail.json) | [mail-write.json](../examples/requests/mail-write.json) | [mail](services/mail.md) |
| `drive` | [drive.schema.json](../schemas/drive.schema.json) | [drive.json](../examples/requests/drive.json) | [drive-write.json](../examples/requests/drive-write.json) | [drive](services/drive.md) |
| `calendar` | [calendar.schema.json](../schemas/calendar.schema.json) | [calendar.json](../examples/requests/calendar.json) | [calendar-write.json](../examples/requests/calendar-write.json) | [calendar](services/calendar.md) |
| `sheets` | [sheets.schema.json](../schemas/sheets.schema.json) | [sheets.json](../examples/requests/sheets.json) | [sheets-write.json](../examples/requests/sheets-write.json) | [sheets](services/sheets.md) |
| `tasks` | [tasks.schema.json](../schemas/tasks.schema.json) | [tasks.json](../examples/requests/tasks.json) | [tasks-write.json](../examples/requests/tasks-write.json) | [tasks](services/tasks.md) |
| `contacts` | [contacts.schema.json](../schemas/contacts.schema.json) | [contacts.json](../examples/requests/contacts.json) | [contacts-write.json](../examples/requests/contacts-write.json) | [contacts](services/contacts.md) |

Use `--help` for common parameters and `--help --verbose` for every parameter.
Unknown parameters and authentication tokens are not accepted in parameter files.

## Advanced request bodies

`body` remains an advanced input for a complete compatibility request object.
Joom translates supported fields to Graph.
Existing request files with a `body` envelope remain accepted. Choose a file
`body` or `--body JSON`; named field options can override individual body fields.
Existing mail `compose` envelopes also remain accepted. Ordinary files use the
flat command parameters shown above.
