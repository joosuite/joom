# JSON request examples

See [Microsoft support](../../docs/microsoft.md) for provider limits.

These files contain Joom CLI parameters. Run them from the repository root with
`--file`, using the matching command below:

```sh
joom mail you@example.com messages.list --file examples/requests/mail.json
joom drive you@example.com files.list --file examples/requests/drive.json
```

Replace `ACCOUNT` with your signed-in account and replace resource IDs such as
`FILE_ID` inside the files with your own IDs. List examples use `"max": 10`
to limit the total number of results across all pages. Explicit command options
override file values, so `--max 5` reduces that limit.

## Read examples

| File | Command |
| --- | --- |
| [mail.json](mail.json) | `joom mail ACCOUNT messages.list` |
| [drive.json](drive.json) | `joom drive ACCOUNT files.list` |
| [calendar.json](calendar.json) | `joom calendar ACCOUNT events.list` |
| [sheets.json](sheets.json) | `joom sheets ACCOUNT values.get` |
| [tasks.json](tasks.json) | `joom tasks ACCOUNT tasks.list` |
| [contacts.json](contacts.json) | `joom contacts ACCOUNT people.connections.list` |

## Write examples

Append `--file examples/requests/FILE.json` to the corresponding command.

| File | Command |
| --- | --- |
| [mail-write.json](mail-write.json) | `joom mail ACCOUNT messages.modify` |
| [drive-write.json](drive-write.json) | `joom drive ACCOUNT permissions.create` |
| [calendar-write.json](calendar-write.json) | `joom calendar ACCOUNT events.insert` |
| [sheets-write.json](sheets-write.json) | `joom sheets ACCOUNT values.update` |
| [tasks-write.json](tasks-write.json) | `joom tasks ACCOUNT tasks.insert` |
| [contacts-write.json](contacts-write.json) | `joom contacts ACCOUNT people.create-contact` |
| [mail-compose.json](mail-compose.json) | `joom mail ACCOUNT messages.send` |

For parameter types, required values, and choices, see the
[JSON parameter file guide](../../docs/file-schemas.md) and command help:

```sh
joom mail ACCOUNT messages.send --help
joom mail ACCOUNT messages.send --help --verbose
```
