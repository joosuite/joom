# Excel methods

See the [command guide](../grammar.md) and [request-file format](../file-schemas.md).

| Method | Description |
| --- | --- |
| `get` | Get Excel workbook title and worksheet metadata. |
| `values.clear` | Clear the contents of an Excel worksheet range. |
| `values.get` | Read values, formatted text, or formulas from an Excel worksheet range. |
| `values.update` | Write rows of values to an Excel worksheet range. |

## get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get Excel workbook title and worksheet metadata.

Usage: joom [OPTIONS] sheets <ACCOUNT> get [COMMAND_OPTIONS]

Mandatory parameters:
      --spreadsheet-id <SPREADSHEET_ID>
          spreadsheetId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

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

      --ttl <MINUTES>
          Reuse encrypted response data; 0 disables caching
          
          [default: 0]

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## values.clear

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Clear the contents of an Excel worksheet range.

Usage: joom [OPTIONS] sheets <ACCOUNT> values.clear [COMMAND_OPTIONS]

Mandatory parameters:
      --range <RANGE>
          range for the Microsoft operation.

      --spreadsheet-id <SPREADSHEET_ID>
          spreadsheetId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

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

      --body <JSON>
          Inline JSON request body

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## values.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Read values, formatted text, or formulas from an Excel worksheet range.

Usage: joom [OPTIONS] sheets <ACCOUNT> values.get [COMMAND_OPTIONS]

Mandatory parameters:
      --range <RANGE>
          range for the Microsoft operation.

      --spreadsheet-id <SPREADSHEET_ID>
          spreadsheetId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --value-render-option <VALUE_RENDER_OPTION>
          valueRenderOption for the Microsoft operation. Choices: FORMATTED_VALUE, UNFORMATTED_VALUE, FORMULA.

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

## values.update

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Write rows of values to an Excel worksheet range.

Usage: joom [OPTIONS] sheets <ACCOUNT> values.update [COMMAND_OPTIONS]

Mandatory parameters:
      --range <RANGE>
          range for the Microsoft operation.

      --spreadsheet-id <SPREADSHEET_ID>
          spreadsheetId for the Microsoft operation.

      --values <JSON>
          Request body field

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

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

      --body <JSON>
          Inline JSON request body

      --major-dimension <MAJOR_DIMENSION>
          Request body field Choices: ROWS.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

Command parameters retain the compatibility input contract. See [Microsoft support](../microsoft.md) for Graph behavior and limits.
