# Outlook Calendar methods

See the [command guide](../grammar.md) and [request-file format](../file-schemas.md).

| Method | Description |
| --- | --- |
| `calendar-list.get` | Get an Outlook calendar and the mailbox timezone. |
| `calendars.delete` | Delete an Outlook calendar. |
| `calendars.get` | Get an Outlook calendar and the mailbox timezone. |
| `calendars.insert` | Create an Outlook calendar. |
| `calendars.list` | List Outlook calendars and the mailbox timezone. |
| `calendars.patch` | Rename an Outlook calendar. |
| `calendars.update` | Rename an Outlook calendar. |
| `events.delete` | Delete an Outlook event; Outlook sends meeting updates automatically. |
| `events.get` | Get an Outlook event. |
| `events.insert` | Create an Outlook event. |
| `events.instances` | List instances of a recurring Outlook event within a time window. |
| `events.list` | List Outlook events or a calendar view within a time window. |
| `events.patch` | Update supported fields of an Outlook event. |
| `events.update` | Update supported fields of an Outlook event. |

## calendar-list.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get an Outlook calendar and the mailbox timezone.

Usage: joom [OPTIONS] calendar <ACCOUNT> calendar-list.get [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --verbose
          Include advanced parameters in help

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --ttl <MINUTES>
          Reuse encrypted response data; 0 disables caching
          
          [default: 0]

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## calendars.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete an Outlook calendar.

Usage: joom [OPTIONS] calendar <ACCOUNT> calendars.delete [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## calendars.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get an Outlook calendar and the mailbox timezone.

Usage: joom [OPTIONS] calendar <ACCOUNT> calendars.get [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --verbose
          Include advanced parameters in help

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --ttl <MINUTES>
          Reuse encrypted response data; 0 disables caching
          
          [default: 0]

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## calendars.insert

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create an Outlook calendar.

Usage: joom [OPTIONS] calendar <ACCOUNT> calendars.insert [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --summary <SUMMARY>
          Request body field

      --verbose
          Include advanced parameters in help

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --body <JSON>
          Inline JSON request body

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## calendars.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List Outlook calendars and the mailbox timezone.

Usage: joom [OPTIONS] calendar <ACCOUNT> calendars.list [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --max <N>
          Return at most N results; by default, return all results

      --verbose
          Include advanced parameters in help

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --ttl <MINUTES>
          Reuse encrypted response data; 0 disables caching
          
          [default: 0]

      --max-pages <N>
          Stop after N pages; by default, fetch all pages

      --max-results <MAX_RESULTS>
          maxResults for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## calendars.patch

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Rename an Outlook calendar.

Usage: joom [OPTIONS] calendar <ACCOUNT> calendars.patch [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --verbose
          Include advanced parameters in help

      --summary <SUMMARY>
          Request body field

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --body <JSON>
          Inline JSON request body

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## calendars.update

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Rename an Outlook calendar.

Usage: joom [OPTIONS] calendar <ACCOUNT> calendars.update [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --verbose
          Include advanced parameters in help

      --summary <SUMMARY>
          Request body field

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --body <JSON>
          Inline JSON request body

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## events.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete an Outlook event; Outlook sends meeting updates automatically.

Usage: joom [OPTIONS] calendar <ACCOUNT> events.delete [COMMAND_OPTIONS]

Mandatory parameters:
      --event-id <EVENT_ID>
          eventId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --send-updates <SEND_UPDATES>
          sendUpdates for the Microsoft operation. Choices: all.

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## events.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get an Outlook event.

Usage: joom [OPTIONS] calendar <ACCOUNT> events.get [COMMAND_OPTIONS]

Mandatory parameters:
      --event-id <EVENT_ID>
          eventId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --verbose
          Include advanced parameters in help

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --ttl <MINUTES>
          Reuse encrypted response data; 0 disables caching
          
          [default: 0]

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## events.insert

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create an Outlook event.

Input choices:
  Start is required: use --start-date for an all-day event or --start-time for a timed event.
  End is required: use --end-date for an all-day event or --end-time for a timed event.

Usage: joom [OPTIONS] calendar <ACCOUNT> events.insert [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --attendees <JSON>
          Request body field

      --verbose
          Include advanced parameters in help

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --description <DESCRIPTION>
          Request body field

      --end-date <END_DATE>
          Request body field

      --end-time <END_TIME>
          Request body field

      --end-time-zone <END_TIME_ZONE>
          Request body field

      --send-updates <SEND_UPDATES>
          sendUpdates for the Microsoft operation. Choices: all.

      --start-date <START_DATE>
          Request body field

      --start-time <START_TIME>
          Request body field

      --start-time-zone <START_TIME_ZONE>
          Request body field

      --summary <SUMMARY>
          Request body field

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --body <JSON>
          Inline JSON request body

      --end <JSON>
          Request body field

      --location <LOCATION>
          Request body field

      --reminders <JSON>
          Request body field

      --reminders-overrides <JSON>
          Request body field

      --reminders-use-default[=<BOOL>]
          Request body field

      --start <JSON>
          Request body field

      --transparency <TRANSPARENCY>
          Request body field Choices: opaque, transparent.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## events.instances

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List instances of a recurring Outlook event within a time window.

Usage: joom [OPTIONS] calendar <ACCOUNT> events.instances [COMMAND_OPTIONS]

Mandatory parameters:
      --event-id <EVENT_ID>
          eventId for the Microsoft operation.

      --time-max <TIME_MAX>
          timeMax for the Microsoft operation.

      --time-min <TIME_MIN>
          timeMin for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --max <N>
          Return at most N results; by default, return all results

      --verbose
          Include advanced parameters in help

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --ttl <MINUTES>
          Reuse encrypted response data; 0 disables caching
          
          [default: 0]

      --max-pages <N>
          Stop after N pages; by default, fetch all pages

      --max-results <MAX_RESULTS>
          maxResults for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## events.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List Outlook events or a calendar view within a time window.

Usage: joom [OPTIONS] calendar <ACCOUNT> events.list [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --max <N>
          Return at most N results; by default, return all results

      --verbose
          Include advanced parameters in help

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --order-by <ORDER_BY>
          orderBy for the Microsoft operation. Choices: startTime.

      --q <Q>
          Search expression supported by the Microsoft adapter.

      --time-max <TIME_MAX>
          timeMax for the Microsoft operation.

      --time-min <TIME_MIN>
          timeMin for the Microsoft operation.

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --ttl <MINUTES>
          Reuse encrypted response data; 0 disables caching
          
          [default: 0]

      --max-pages <N>
          Stop after N pages; by default, fetch all pages

      --max-results <MAX_RESULTS>
          maxResults for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## events.patch

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Update supported fields of an Outlook event.

Input choices:
  When setting start, use --start-date for an all-day event or --start-time for a timed event.
  When setting end, use --end-date for an all-day event or --end-time for a timed event.

Usage: joom [OPTIONS] calendar <ACCOUNT> events.patch [COMMAND_OPTIONS]

Mandatory parameters:
      --event-id <EVENT_ID>
          eventId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --attendees <JSON>
          Request body field

      --verbose
          Include advanced parameters in help

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --description <DESCRIPTION>
          Request body field

      --end-date <END_DATE>
          Request body field

      --end-time <END_TIME>
          Request body field

      --end-time-zone <END_TIME_ZONE>
          Request body field

      --send-updates <SEND_UPDATES>
          sendUpdates for the Microsoft operation. Choices: all.

      --start-date <START_DATE>
          Request body field

      --start-time <START_TIME>
          Request body field

      --start-time-zone <START_TIME_ZONE>
          Request body field

      --summary <SUMMARY>
          Request body field

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --body <JSON>
          Inline JSON request body

      --end <JSON>
          Request body field

      --location <LOCATION>
          Request body field

      --reminders <JSON>
          Request body field

      --reminders-overrides <JSON>
          Request body field

      --reminders-use-default[=<BOOL>]
          Request body field

      --start <JSON>
          Request body field

      --transparency <TRANSPARENCY>
          Request body field Choices: opaque, transparent.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## events.update

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Update supported fields of an Outlook event.

Input choices:
  When setting start, use --start-date for an all-day event or --start-time for a timed event.
  When setting end, use --end-date for an all-day event or --end-time for a timed event.

Usage: joom [OPTIONS] calendar <ACCOUNT> events.update [COMMAND_OPTIONS]

Mandatory parameters:
      --event-id <EVENT_ID>
          eventId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --attendees <JSON>
          Request body field

      --verbose
          Include advanced parameters in help

      --calendar-id <CALENDAR_ID>
          Outlook calendar ID; primary selects the default calendar.
          
          [default: primary]

      --description <DESCRIPTION>
          Request body field

      --end-date <END_DATE>
          Request body field

      --end-time <END_TIME>
          Request body field

      --end-time-zone <END_TIME_ZONE>
          Request body field

      --send-updates <SEND_UPDATES>
          sendUpdates for the Microsoft operation. Choices: all.

      --start-date <START_DATE>
          Request body field

      --start-time <START_TIME>
          Request body field

      --start-time-zone <START_TIME_ZONE>
          Request body field

      --summary <SUMMARY>
          Request body field

  -h, --help
          Print help (see a summary with '-h')

Optional parameters (advanced):
      --home <PATH>
          Store state in PATH/joom Place before the command.

      --fields <MASK>
          Select response fields

      --timeout <SECONDS>
          Microsoft request timeout in seconds Place before the command.
          
          [default: 30]

      --body <JSON>
          Inline JSON request body

      --end <JSON>
          Request body field

      --location <LOCATION>
          Request body field

      --reminders <JSON>
          Request body field

      --reminders-overrides <JSON>
          Request body field

      --reminders-use-default[=<BOOL>]
          Request body field

      --start <JSON>
          Request body field

      --transparency <TRANSPARENCY>
          Request body field Choices: opaque, transparent.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

Command parameters retain the compatibility input contract. See [Microsoft support](../microsoft.md) for Graph behavior and limits.
