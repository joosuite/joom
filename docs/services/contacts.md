# Outlook Contacts methods

See the [command guide](../grammar.md) and [request-file format](../file-schemas.md).

| Method | Description |
| --- | --- |
| `people.connections.list` | List the signed-in account Outlook contacts. |
| `people.create-contact` | Create an Outlook contact. |
| `people.delete-contact` | Delete an Outlook contact. |
| `people.get` | Get an Outlook contact. |
| `people.search-contacts` | Find Outlook contacts by name prefix. |
| `people.update-contact` | Update supported fields of an Outlook contact. |

## people.connections.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List the signed-in account Outlook contacts.

Usage: joom [OPTIONS] contacts <ACCOUNT> people.connections.list [COMMAND_OPTIONS]

Mandatory parameters:
      --resource-name <RESOURCE_NAME>
          Signed-in Outlook contacts; use people/me.

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

      --page-size <PAGE_SIZE>
          pageSize for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## people.create-contact

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create an Outlook contact.

Usage: joom [OPTIONS] contacts <ACCOUNT> people.create-contact [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --email-addresses <JSON>
          Request body field

      --names <JSON>
          Request body field

      --organizations <JSON>
          Request body field

      --phone-numbers <JSON>
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

      --biographies <JSON>
          Request body field

      --etag <ETAG>
          Request body field

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## people.delete-contact

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete an Outlook contact.

Usage: joom [OPTIONS] contacts <ACCOUNT> people.delete-contact [COMMAND_OPTIONS]

Mandatory parameters:
      --resource-name <RESOURCE_NAME>
          Outlook contact resource ID.

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

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## people.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get an Outlook contact.

Usage: joom [OPTIONS] contacts <ACCOUNT> people.get [COMMAND_OPTIONS]

Mandatory parameters:
      --resource-name <RESOURCE_NAME>
          Outlook contact resource ID.

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

## people.search-contacts

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Find Outlook contacts by name prefix.

Usage: joom [OPTIONS] contacts <ACCOUNT> people.search-contacts [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --max <N>
          Return at most N results; by default, return all results

      --verbose
          Include advanced parameters in help

      --query <QUERY>
          query for the Microsoft operation.

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

      --page-size <PAGE_SIZE>
          pageSize for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## people.update-contact

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Update supported fields of an Outlook contact.

Usage: joom [OPTIONS] contacts <ACCOUNT> people.update-contact [COMMAND_OPTIONS]

Mandatory parameters:
      --resource-name <RESOURCE_NAME>
          Outlook contact resource ID.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --email-addresses <JSON>
          Request body field

      --names <JSON>
          Request body field

      --organizations <JSON>
          Request body field

      --phone-numbers <JSON>
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

      --biographies <JSON>
          Request body field

      --etag <ETAG>
          Request body field

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

Command parameters retain the compatibility input contract. See [Microsoft support](../microsoft.md) for Graph behavior and limits.
