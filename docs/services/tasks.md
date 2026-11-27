# Microsoft To Do methods

See the [command guide](../grammar.md) and [request-file format](../file-schemas.md).

| Method | Description |
| --- | --- |
| `tasklists.delete` | Delete a Microsoft To Do list. |
| `tasklists.get` | Get a Microsoft To Do list. |
| `tasklists.insert` | Create a Microsoft To Do list. |
| `tasklists.list` | List Microsoft To Do lists. |
| `tasklists.patch` | Update a Microsoft To Do list. |
| `tasklists.update` | Update a Microsoft To Do list. |
| `tasks.delete` | Delete a Microsoft To Do task. |
| `tasks.get` | Get a Microsoft To Do task. |
| `tasks.insert` | Create a Microsoft To Do task. |
| `tasks.list` | List Microsoft To Do tasks. |
| `tasks.patch` | Update a Microsoft To Do task. |
| `tasks.update` | Update a Microsoft To Do task. |

## tasklists.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete a Microsoft To Do list.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasklists.delete [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

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

## tasklists.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get a Microsoft To Do list.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasklists.get [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

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

## tasklists.insert

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create a Microsoft To Do list.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasklists.insert [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --title <TITLE>
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

## tasklists.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List Microsoft To Do lists.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasklists.list [COMMAND_OPTIONS]

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

## tasklists.patch

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Update a Microsoft To Do list.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasklists.patch [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

      --verbose
          Include advanced parameters in help

      --title <TITLE>
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

## tasklists.update

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Update a Microsoft To Do list.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasklists.update [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

      --verbose
          Include advanced parameters in help

      --title <TITLE>
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

## tasks.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete a Microsoft To Do task.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasks.delete [COMMAND_OPTIONS]

Mandatory parameters:
      --task <TASK>
          task for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

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

## tasks.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get a Microsoft To Do task.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasks.get [COMMAND_OPTIONS]

Mandatory parameters:
      --task <TASK>
          task for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

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

## tasks.insert

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create a Microsoft To Do task.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasks.insert [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --due <DUE>
          Request body field

      --notes <NOTES>
          Request body field

      --status <STATUS>
          Request body field Choices: needsAction, completed.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

      --title <TITLE>
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

      --completed <COMPLETED>
          Request body field

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## tasks.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List Microsoft To Do tasks.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasks.list [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --max <N>
          Return at most N results; by default, return all results

      --verbose
          Include advanced parameters in help

      --show-completed[=<BOOL>]
          showCompleted for the Microsoft operation.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

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

      --due-max <DUE_MAX>
          dueMax for the Microsoft operation.

      --due-min <DUE_MIN>
          dueMin for the Microsoft operation.

      --max-results <MAX_RESULTS>
          maxResults for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## tasks.patch

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Update a Microsoft To Do task.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasks.patch [COMMAND_OPTIONS]

Mandatory parameters:
      --task <TASK>
          task for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --due <DUE>
          Request body field

      --notes <NOTES>
          Request body field

      --status <STATUS>
          Request body field Choices: needsAction, completed.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

      --title <TITLE>
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

      --completed <COMPLETED>
          Request body field

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## tasks.update

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Update a Microsoft To Do task.

Usage: joom [OPTIONS] tasks <ACCOUNT> tasks.update [COMMAND_OPTIONS]

Mandatory parameters:
      --task <TASK>
          task for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --due <DUE>
          Request body field

      --notes <NOTES>
          Request body field

      --status <STATUS>
          Request body field Choices: needsAction, completed.

      --tasklist <TASKLIST>
          tasklist for the Microsoft operation.
          
          [default: @default]

      --title <TITLE>
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

      --completed <COMPLETED>
          Request body field

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

Command parameters retain the compatibility input contract. See [Microsoft support](../microsoft.md) for Graph behavior and limits.
