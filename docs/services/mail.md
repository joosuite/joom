# Outlook Mail methods

See the [command guide](../grammar.md) and [request-file format](../file-schemas.md).

| Method | Description |
| --- | --- |
| `drafts.create` | Create an Outlook draft from composed or raw MIME content. |
| `drafts.delete` | Delete an Outlook draft. |
| `drafts.get` | Read an Outlook draft. |
| `drafts.list` | List Outlook drafts. |
| `drafts.send` | Send an existing Outlook draft. |
| `labels.create` | Create a top-level Outlook mail folder. |
| `labels.delete` | Delete an Outlook mail folder. |
| `labels.get` | Get an Outlook mail folder. |
| `labels.list` | List top-level Outlook mail folders. |
| `labels.patch` | Rename an Outlook mail folder. |
| `labels.update` | Rename an Outlook mail folder. |
| `messages.attachments.get` | Get the content of an Outlook file attachment. |
| `messages.delete` | Permanently delete an Outlook message. |
| `messages.get` | Read an Outlook message and its attachment metadata. |
| `messages.list` | List or search Outlook messages. |
| `messages.modify` | Change the read or flagged state of an Outlook message. |
| `messages.send` | Send composed or raw MIME content through Outlook. |
| `messages.trash` | Move an Outlook message to Deleted Items. |
| `messages.untrash` | Move an Outlook message to the inbox. |
| `profile` | Get the signed-in Microsoft account email address. |

## drafts.create

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create an Outlook draft from composed or raw MIME content.

Input choices:
  Use --text for inline content or --text-file to read a file.
  Use --html for inline content or --html-file to read a file.
  Compose a message with recipients and content, or supply raw MIME. Composition cannot be combined with raw message content or --upload.

Usage: joom [OPTIONS] mail <ACCOUNT> drafts.create [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --attach <PATH>...
          Attach local files as application/octet-stream. Repeat --attach for multiple files.

      --json
          Output JSON instead of human-readable text Place before the command.

      --bcc <EMAIL>...
          Bcc email addresses. Repeat --bcc for multiple recipients.

      --verbose
          Include advanced parameters in help

      --cc <EMAIL>...
          Cc email addresses. Repeat --cc for multiple recipients.

      --from <EMAIL>
          Sender email address; defaults to ACCOUNT. Microsoft must allow this sender.

      --html <HTML>
          HTML message body; combines with text as multipart/alternative.

      --html-file <PATH>
          Read the HTML body from a UTF-8 file.

      --reply-to <EMAIL>
          Reply-To email address.

      --subject <TEXT>
          Message subject.

      --text <TEXT>
          Plain text message body.

      --text-file <PATH>
          Read the plain text body from a UTF-8 file.

      --to <EMAIL>...
          Recipient email addresses. Repeat --to for multiple recipients.

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

      --message <JSON>
          Request body field

      --message-raw <MESSAGE_RAW>
          Request body field

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## drafts.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete an Outlook draft.

Usage: joom [OPTIONS] mail <ACCOUNT> drafts.delete [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## drafts.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Read an Outlook draft.

Usage: joom [OPTIONS] mail <ACCOUNT> drafts.get [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## drafts.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List Outlook drafts.

Usage: joom [OPTIONS] mail <ACCOUNT> drafts.list [COMMAND_OPTIONS]

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

      --max-results <MAX_RESULTS>
          maxResults for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## drafts.send

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Send an existing Outlook draft.

Usage: joom [OPTIONS] mail <ACCOUNT> drafts.send [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
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

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## labels.create

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Create a top-level Outlook mail folder.

Usage: joom [OPTIONS] mail <ACCOUNT> labels.create [COMMAND_OPTIONS]

Mandatory parameters:
      --name <NAME>
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

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## labels.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Delete an Outlook mail folder.

Usage: joom [OPTIONS] mail <ACCOUNT> labels.delete [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## labels.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get an Outlook mail folder.

Usage: joom [OPTIONS] mail <ACCOUNT> labels.get [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## labels.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List top-level Outlook mail folders.

Usage: joom [OPTIONS] mail <ACCOUNT> labels.list [COMMAND_OPTIONS]

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

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## labels.patch

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Rename an Outlook mail folder.

Usage: joom [OPTIONS] mail <ACCOUNT> labels.patch [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

      --name <NAME>
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

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## labels.update

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Rename an Outlook mail folder.

Usage: joom [OPTIONS] mail <ACCOUNT> labels.update [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

      --name <NAME>
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

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## messages.attachments.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get the content of an Outlook file attachment.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.attachments.get [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

      --message-id <MESSAGE_ID>
          messageId for the Microsoft operation.

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

## messages.delete

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Permanently delete an Outlook message.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.delete [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## messages.get

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Read an Outlook message and its attachment metadata.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.get [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## messages.list

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
List or search Outlook messages.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.list [COMMAND_OPTIONS]

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

      --label-ids <LABEL_IDS>...
          labelIds for the Microsoft operation. Choices: INBOX, UNREAD.

      --max-results <MAX_RESULTS>
          maxResults for the Microsoft operation.

      --page-token <PAGE_TOKEN>
          pageToken for the Microsoft operation.

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## messages.modify

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Change the read or flagged state of an Outlook message.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.modify [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --json
          Output JSON instead of human-readable text Place before the command.

      --add-label-ids <ADD_LABEL_IDS>...
          Request body field

      --verbose
          Include advanced parameters in help

      --remove-label-ids <REMOVE_LABEL_IDS>...
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

## messages.send

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Send composed or raw MIME content through Outlook.

Input choices:
  Use --text for inline content or --text-file to read a file.
  Use --html for inline content or --html-file to read a file.
  Compose a message with recipients and content, or supply raw MIME. Composition cannot be combined with raw message content or --upload.
  When composing a message to send, provide at least one recipient with --to, --cc, or --bcc.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.send [COMMAND_OPTIONS]

Optional parameters:
      --file <PATH>
          Read command parameters from a JSON file

      --attach <PATH>...
          Attach local files as application/octet-stream. Repeat --attach for multiple files.

      --json
          Output JSON instead of human-readable text Place before the command.

      --bcc <EMAIL>...
          Bcc email addresses. Repeat --bcc for multiple recipients.

      --verbose
          Include advanced parameters in help

      --cc <EMAIL>...
          Cc email addresses. Repeat --cc for multiple recipients.

      --from <EMAIL>
          Sender email address; defaults to ACCOUNT. Microsoft must allow this sender.

      --html <HTML>
          HTML message body; combines with text as multipart/alternative.

      --html-file <PATH>
          Read the HTML body from a UTF-8 file.

      --reply-to <EMAIL>
          Reply-To email address.

      --subject <TEXT>
          Message subject.

      --text <TEXT>
          Plain text message body.

      --text-file <PATH>
          Read the plain text body from a UTF-8 file.

      --to <EMAIL>...
          Recipient email addresses. Repeat --to for multiple recipients.

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

      --raw <RAW>
          Request body field

Help:
  Use --help --verbose to see all parameters and full descriptions.
  Read parameters from --file PATH; explicit options override file values.
```

## messages.trash

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Move an Outlook message to Deleted Items.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.trash [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## messages.untrash

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Move an Outlook message to the inbox.

Usage: joom [OPTIONS] mail <ACCOUNT> messages.untrash [COMMAND_OPTIONS]

Mandatory parameters:
      --id <ID>
          id for the Microsoft operation.

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

## profile

Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.

```text
Get the signed-in Microsoft account email address.

Usage: joom [OPTIONS] mail <ACCOUNT> profile [COMMAND_OPTIONS]

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

Command parameters retain the compatibility input contract. See [Microsoft support](../microsoft.md) for Graph behavior and limits.
