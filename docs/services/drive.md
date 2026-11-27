# OneDrive methods

See the [command guide](../grammar.md) and [request-file format](../file-schemas.md).

| Method | Description |
| --- | --- |
| `drives.get` | Get a Microsoft drive. |
| `drives.list` | List the signed-in account drives. |
| `files.copy` | Copy a OneDrive file through download and upload; folders are not supported. |
| `files.create` | Create a OneDrive folder or upload a local file. |
| `files.delete` | Move a OneDrive item to the recycle bin. |
| `files.export` | Export an existing OneDrive file to PDF. |
| `files.get` | Get OneDrive item metadata or download its content. |
| `files.list` | List or search OneDrive items in a folder. |
| `files.update` | Rename, move, recycle, or upload new content to a OneDrive item. |
| `permissions.create` | Invite a user to read or edit a OneDrive item. |
| `permissions.delete` | Delete a OneDrive item permission. |
| `permissions.get` | Get a OneDrive item permission. |
| `permissions.list` | List OneDrive item permissions. |
| `revisions.get` | Get or download a OneDrive file version. |
| `revisions.list` | List OneDrive file versions. |

## drives.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get a Microsoft drive.

Usage: joom [OPTIONS] drive <ACCOUNT> drives.get [COMMAND_OPTIONS]

Mandatory parameters:
      --drive-id <DRIVE_ID>
          driveId for the Microsoft operation.

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

## drives.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List the signed-in account drives.

Usage: joom [OPTIONS] drive <ACCOUNT> drives.list [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --max <N>
          Return at most N results; by default, return all results

      --verbose
          Include advanced parameters in help

      --q <Q>
          Search expression supported by the Microsoft adapter.

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

## files.copy

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Copy a OneDrive file through download and upload; folders are not supported.

Usage: joom [OPTIONS] drive <ACCOUNT> files.copy [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --verbose
          Include advanced parameters in help

      --name <NAME>
          Request body field

      --parents <PARENTS>...
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

## files.create

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create a OneDrive folder or upload a local file.

Input choices:
  --content-type sets the MIME type of a file supplied with --upload.

Usage: joom [OPTIONS] drive <ACCOUNT> files.create [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --upload <PATH>
          Upload a local file of at most 250 MiB.

      --verbose
          Include advanced parameters in help

      --description <DESCRIPTION>
          Request body field

      --mime-type <MIME_TYPE>
          Request body field Choices: inode/directory.

      --name <NAME>
          Request body field

      --parents <PARENTS>...
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

      --content-type <MIME>
          Upload MIME type
          
          [default: application/octet-stream]

      --body <JSON>
          Inline JSON request body

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## files.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Move a OneDrive item to the recycle bin.

Usage: joom [OPTIONS] drive <ACCOUNT> files.delete [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

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

## files.export

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Export an existing OneDrive file to PDF.

Usage: joom [OPTIONS] drive <ACCOUNT> files.export [COMMAND_OPTIONS]

Mandatory parameters:
  -o, --output <PATH>
          Save the downloaded content to a new file

      --file-id <FILE_ID>
          fileId for the Microsoft operation.

      --mime-type <MIME_TYPE>
          mimeType for the Microsoft operation. Choices: application/pdf.

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

## files.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get OneDrive item metadata or download its content.

Usage: joom [OPTIONS] drive <ACCOUNT> files.get [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

  -o, --output <PATH>
          Save the downloaded content to a new file

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

## files.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List or search OneDrive items in a folder.

Usage: joom [OPTIONS] drive <ACCOUNT> files.list [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --max <N>
          Return at most N results; by default, return all results

      --verbose
          Include advanced parameters in help

      --order-by <ORDER_BY>
          orderBy for the Microsoft operation. Choices: name, name desc, modifiedTime, modifiedTime desc, createdTime, createdTime desc.

      --q <Q>
          Search expression supported by the Microsoft adapter.

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

      --drive-id <DRIVE_ID>
          driveId for the Microsoft operation.

      --page-size <PAGE_SIZE>
          pageSize for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## files.update

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Rename, move, recycle, or upload new content to a OneDrive item.

Input choices:
  --content-type sets the MIME type of a file supplied with --upload.

Usage: joom [OPTIONS] drive <ACCOUNT> files.update [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --upload <PATH>
          Upload a local file of at most 250 MiB.

      --verbose
          Include advanced parameters in help

      --description <DESCRIPTION>
          Request body field

      --name <NAME>
          Request body field

      --parents <PARENTS>...
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

      --content-type <MIME>
          Upload MIME type
          
          [default: application/octet-stream]

      --body <JSON>
          Inline JSON request body

      --add-parents <ADD_PARENTS>
          addParents for the Microsoft operation.

      --remove-parents <REMOVE_PARENTS>
          removeParents for the Microsoft operation.

      --trashed[=<BOOL>]
          Request body field Choices: .

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## permissions.create

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Invite a user to read or edit a OneDrive item.

Usage: joom [OPTIONS] drive <ACCOUNT> permissions.create [COMMAND_OPTIONS]

Mandatory parameters:
      --email-address <EMAIL_ADDRESS>
          Request body field

      --file-id <FILE_ID>
          fileId for the Microsoft operation.

      --role <ROLE>
          Request body field Choices: reader, writer.

      --type <TYPE>
          Request body field Choices: user.

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

      --send-notification-email[=<BOOL>]
          sendNotificationEmail for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## permissions.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete a OneDrive item permission.

Usage: joom [OPTIONS] drive <ACCOUNT> permissions.delete [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

      --permission-id <PERMISSION_ID>
          permissionId for the Microsoft operation.

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

## permissions.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get a OneDrive item permission.

Usage: joom [OPTIONS] drive <ACCOUNT> permissions.get [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

      --permission-id <PERMISSION_ID>
          permissionId for the Microsoft operation.

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

## permissions.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List OneDrive item permissions.

Usage: joom [OPTIONS] drive <ACCOUNT> permissions.list [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

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

## revisions.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get or download a OneDrive file version.

Usage: joom [OPTIONS] drive <ACCOUNT> revisions.get [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

      --revision-id <REVISION_ID>
          revisionId for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

  -o, --output <PATH>
          Save the downloaded content to a new file

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

## revisions.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List OneDrive file versions.

Usage: joom [OPTIONS] drive <ACCOUNT> revisions.list [COMMAND_OPTIONS]

Mandatory parameters:
      --file-id <FILE_ID>
          fileId for the Microsoft operation.

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

Command parameters retain the compatibility input contract. See [Microsoft support](../microsoft.md) for Graph behavior and limits.
