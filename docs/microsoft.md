# Microsoft operations

Joom provides flat commands, wrappers, JSON request files, and plain or JSON output.
The provider adapter translates requests and responses underneath those commands.
Bundled contracts use Microsoft Graph origins and service identifiers.
Authenticated API requests go to Microsoft Graph.

## Word, Excel, and PowerPoint

Excel worksheet metadata, range reads, range writes, and range clearing use
Microsoft Graph workbook endpoints. Microsoft documents these endpoints for
OneDrive for Business, SharePoint, and group drives, and excludes consumer
OneDrive in the [Excel Graph API reference](https://learn.microsoft.com/en-us/graph/api/resources/excel?view=graph-rest-1.0).
The recorded live checks succeeded for both approved personal accounts'
workbook operations. Those account-specific results do not establish support
for other consumer storage.

Word and PowerPoint wrappers use OneDrive for file listing and PDF export.
Word file sharing uses OneDrive permissions.

## Other services

| Command group | Implemented operations |
| --- | --- |
| `mail` | List and read messages, send MIME mail, create/read/send/delete drafts, mark read or starred, trash/untrash, permanent delete, download file attachments, manage top-level mail folders through label commands |
| `drive` | List/search a folder, read metadata, create folders, copy files, rename/move items, recycle items, upload/download files, PDF export, inspect versions and permissions, invite a user |
| `calendar` | List calendars, create/read/rename/delete calendars, list/read/create/update/delete events, list recurring-event instances |
| `tasks` | List/create/read/update/delete To Do lists and tasks, resolve `@default` to the default list |
| `contacts` | List/search/create/read/update/delete Outlook contacts |

Mail folders map to label management commands. Each message belongs to one
Outlook folder. Message label mutations currently support
`UNREAD` and `STARRED`. Outlook cannot combine every search expression and
filter. Unsupported combinations fail explicitly.

OneDrive file lists use folder children, or search within the selected folder.
An omitted parent selects the root. MIME filters are applied locally to the
returned collection. File uploads use OneDrive's simple-upload endpoint, limited
to 250 MiB. Downloads follow the returned HTTPS content URL without forwarding
the Graph bearer token.

Calendar timezone reads use the account's mailbox settings. Windows timezone
names are converted to IANA names. Calendar wrappers use the computer's timezone
when the mailbox has no timezone setting. An explicit `timezone:` value takes
precedence over both. Outlook sends meeting updates automatically.
Requests to suppress those updates are rejected. RFC5545 recurrence writes and
Additional calendar operations are absent from help and schemas.

Only implemented operations are exposed in command help and schemas.
Unsupported commands and fields fail local validation. Store photo files through
the drive commands.

The implementation follows Microsoft's references for
[mail](https://learn.microsoft.com/en-us/graph/api/user-list-messages?view=graph-rest-1.0),
[sending messages](https://learn.microsoft.com/en-us/graph/api/user-sendmail?view=graph-rest-1.0),
[calendar events](https://learn.microsoft.com/en-us/graph/api/calendar-list-events?view=graph-rest-1.0),
[contacts](https://learn.microsoft.com/en-us/graph/api/user-list-contacts?view=graph-rest-1.0),
and [To Do](https://learn.microsoft.com/en-us/graph/api/todotasklist-list-tasks?view=graph-rest-1.0).

## Verification

Local tests cover command contracts, Microsoft request conversion, response
mappings, and rejection of unsupported operations. Explicit live checks record
per-command Microsoft outcomes and cleanup separately from local validation.
See [tests and verification](testing.md) for running them and reading coverage.
