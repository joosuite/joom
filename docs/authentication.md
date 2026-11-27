# Sign-in setup

## Initialize storage

```sh
./bin/joom auth init
```

Joom creates a 256-bit master key in the OS keychain. Running the command again
leaves an existing vault intact without reading its key or changing Keychain
permissions. On macOS, credential reads use Apple's built-in `/usr/bin/security`
tool. Rebuilding Joom does not change that tool's identity. Access to an existing
entry can still require macOS approval.

| Platform | Credential store | Default state directory |
| --- | --- | --- |
| macOS | Keychain through Apple's `security` tool | `~/Library/Application Support/joom` |
| Windows | Windows Credential Manager | `%APPDATA%\joom` |
| Linux | Secret Service over the user's D-Bus session | `$XDG_CONFIG_HOME/joom`, or `~/.config/joom` |

## Register a Microsoft application

1. Open [App registrations in Microsoft Entra](https://entra.microsoft.com/).
2. Register an application with **Accounts in any organizational directory and personal Microsoft accounts**.
3. Under **Authentication**, add a **Mobile and desktop applications** platform.
4. Register `http://localhost/oauth/callback` under **Mobile and desktop applications**. Joom uses an available local port; Microsoft ignores the port when matching a registered `localhost` callback. Use this exact hostname and path.
5. Copy the **Application client ID**.

For the account types above, the app manifest must contain
`"signInAudience": "AzureADandPersonalMicrosoftAccount"`. A value of
`AzureADMyOrg` restricts sign-in to the app's organization and rejects personal
accounts. In the Microsoft Graph manifest format, set
`api.requestedAccessTokenVersion` to `2` for personal-account support. Keep the
other manifest fields intact. See Microsoft's [manifest reference](https://learn.microsoft.com/en-us/entra/identity-platform/reference-microsoft-graph-app-manifest).

Joom uses Microsoft's authorization code flow with PKCE through the `common`
tenant endpoint by default. Use `--personal` during sign-in to use the `consumers`
endpoint for a personal Microsoft account. Its callback listener binds only to
the local loopback address.
See Microsoft's [redirect URI guidance](https://learn.microsoft.com/en-us/entra/identity-platform/reply-url)
and [authorization code flow](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow).

Joom is a native desktop public client. It uses an application ID and PKCE;
it does not request or transmit a client secret. Existing credentials containing
a secret remain readable, but the native sign-in flow ignores that secret.

## Enroll the credentials

```sh
./bin/joom auth enroll
```

Enter the Application (client) ID. The prompt masks input with `*`.
Joom encrypts the credentials locally and prints `Microsoft client credentials saved.`

For automation, `joom auth enroll --stdin` reads a JSON object containing
`client_id` from a protected input source. A legacy `client_secret` field is ignored.

When switching OAuth clients, log out accounts using the previous client,
enroll the new credentials, and sign in again.

## Test the connection

```sh
./bin/joom auth test
```

Joom checks whether Microsoft's OAuth endpoint responds. This does not prove the
application ID exists, the app supports personal accounts, or sign-in will work.
An invalid refresh-token response can occur before Microsoft validates the
application. On success:

```text
Microsoft OAuth endpoint reachable. App registration and sign-in were not verified.
```

## Sign in

```sh
./bin/joom auth you@example.com all
```

`all` is a shortcut for `login all`. It requests all configured permissions for
the configured Microsoft services. To select fewer services, use a comma-separated
list:

```sh
./bin/joom auth you@example.com login mail,drive,calendar
```

Select the matching Microsoft account in the browser and approve the requested
permissions. Complete sign-in within three minutes. Joom saves encrypted tokens
and refreshes them automatically when needed.

For a personal Microsoft account, including one registered with a Gmail address,
you can select Microsoft's personal-account endpoint explicitly:

```sh
./bin/joom auth you@gmail.com login all --personal
```

Joom saves this endpoint choice for token refresh and credential transfer.
It does not infer the account type from the email domain. See Microsoft's
[account endpoint guidance](https://learn.microsoft.com/en-us/entra/identity-platform/msal-client-application-configuration).

The delegated permissions depend on the selected services. `mail` requests
`Mail.ReadWrite` and `Mail.Send`. `calendar` requests `Calendars.ReadWrite` and
`MailboxSettings.Read` for the account timezone. `drive`, `docs`, `sheets`, and
`slides` request `Files.ReadWrite`. `tasks` requests `Tasks.ReadWrite`, and
`contacts` requests `Contacts.ReadWrite`. Joom also requests `User.Read`,
`openid`, `email`, and `offline_access` for sign-in and token refresh.

Sign in again when adding services. An organization's consent policy can require
administrator approval. The [Microsoft operation guide](microsoft.md) lists the
operations implemented by this copy.

## List registered accounts

```sh
./bin/joom auth list
./bin/joom --json auth list
```

The default output lists one account email per line. JSON output contains an
array of the same emails. Use `--home PATH` to list accounts in another vault.
Listing does not unlock credentials or contact Microsoft. It shows registrations;
Microsoft permissions may still have expired or been revoked. An empty vault
produces no text, or `[]` in JSON.

## Run commands

```sh
./bin/joom drive you@example.com files.list --max 5
./bin/joom --json drive you@example.com files.list --max 5
./bin/joom mail you@example.com messages.list --q is:unread
```

See the [command reference](grammar.md) and [parameter file schemas](file-schemas.md).

## Use Joom on a VPS

Sign in on your laptop, then export the account:

```sh
./bin/joom auth you@example.com all
./bin/joom auth you@example.com export credentials.joom
scp credentials.joom user@vps:~/
```

Export includes the OAuth client ID and the account's refresh
token. Choose a transfer passphrase with at least 12 characters when prompted.
The file is encrypted with AES-256-GCM using a key derived with
[Argon2id](https://docs.rs/argon2/0.5.3/argon2/). Existing export files are never
overwritten. Omit the account to export all signed-in accounts:

```sh
./bin/joom auth export credentials-all.joom
```

On the VPS, import the file and enter the same passphrase:

```sh
joom auth import credentials.joom --headless
joom mail you@example.com messages.list --max 5
```

Import needs no browser, Microsoft request, or prior `auth enroll`. `--headless`
creates a vault with a local `master.key` file instead of using a desktop
keychain. The vault directory uses mode `0700` and the key uses `0600` on Unix.
Protect that directory as you would an SSH private key. Subsequent commands and
wrappers select this storage automatically. Access tokens refresh when you next
use a service, using the permissions already granted on the laptop.

Omit `--headless` when importing to a desktop with an OS keychain. To create an
empty headless vault separately, use `joom auth init --headless`. An existing
keychain vault is never converted to file storage; choose a separate directory
with `--home PATH` if needed.

Import merges accounts into the destination vault. Existing account credentials
require `--replace`. An import with different OAuth client credentials cannot
replace the client while leaving accounts that depend on the old client. Use a
separate vault in that case. Add `ACCOUNT` before `import` to select one account
from a multi-account export.

For automation, `--passphrase-file PATH` reads the passphrase from a private
UTF-8 file, with an optional final newline. On Unix, its permissions must exclude
other users. See `auth export --help --verbose` and `auth import --help --verbose`.
Passphrases are never accepted as command-line values or written to the vault.

After importing, remove the transfer files from both machines when you no longer
need them. `auth logout` removes only the local credentials. Revoking the Microsoft
grant affects both machines.

## Sign out

```sh
./bin/joom auth you@example.com logout
```

Joom deletes that account's local tokens. To revoke its Microsoft permissions,
remove the grant in your Microsoft Account settings.

## Stored data

| Data | Storage |
| --- | --- |
| Master key | OS keychain, or private `master.key` with `--headless` |
| Application client ID | Encrypted `oauth-client` record |
| Account tokens | Encrypted record per account |
| Authorized account emails | `config.json` |
| Cached Microsoft responses | Encrypted account cache when `--ttl` is positive |

`--home PATH` selects `PATH/joom` and a separate vault.

The default Keychain service is `joom/vault`, with account `master-key`.
Custom vaults have a random identifier saved once in `vault.json`. Moving a vault
preserves that identifier. Builds never create a new vault identity.
On Windows, Credential Manager displays the default entry as
`master-key.joom/vault`. On Linux, its Secret Service attributes are
`service=joom/vault` and `username=master-key`.
Credential files are under `secrets/`. Cached responses are under
`responses/ACCOUNT_HASH/`.

Existing `joom-rust` directories, or `PATH/joom-rust` with `--home`, stay in place.
Joom reads their existing keys without copying or renaming them. Normal commands
and `auth init` do not migrate storage or change Keychain permissions. If both
directories contain state, Joom stops without choosing or changing either vault.

Windows and Linux read keys through their native credential stores.
No signing certificate is required. Each command reads the master key once;
wrappers reuse it for all their API operations. An OS store that is locked or
unavailable still needs to be unlocked or made available. On Linux, keychain
storage requires a running Secret Service on the user's D-Bus session. Headless
vaults use their private key file and do not contact Secret Service.

Credentials and cached account data use AES-256-GCM encryption. On Unix,
directories use mode `0700` and credential files use mode `0600`.

Caching is disabled by default. `--ttl 60` keeps read responses for up to an
hour. Each account's cache holds at most 32 entries and 64 MiB. Expired or damaged
entries are refetched from Microsoft.

## Troubleshooting

### macOS approval after a rebuild

New Joom Keychain entries trust `/usr/bin/security`, which Apple signs and supplies
with macOS. Access approval therefore belongs to that stable tool instead of
each rebuilt Joom executable. No local signing certificate is needed.

Existing entries keep their original access rules. Reading them through the
system tool may still require macOS approval. Joom does not rewrite those rules.

The master key travels through private process pipes, never through command-line
arguments, environment variables, or temporary files. Keychain entries do not
allow every application. A process running as your macOS user can invoke the
trusted `security` tool to access the entry.

Within a command, Joom keeps one master-key session in memory and clears it when
the process exits. Wrappers reuse that session for their API operations. A locked
login Keychain can still require unlocking. If an older build asks for access as
`joom`, rebuild with `make build` and use the new binary and its wrappers.

### Error messages

| Error | Action |
| --- | --- |
| Keychain unavailable or locked | Unlock the OS keychain and retry. |
| Master key missing | Restore the original keychain entry. A new key cannot decrypt existing records. |
| Credential authentication failed | Check the vault path and keychain entry, or restore intact credential files. |
| Client credentials missing | Run `joom auth enroll`. |
| Redirect URI mismatch | Register http://localhost/oauth/callback under Mobile and desktop applications. |
| Browser authorization timed out | Retry login and complete sign-in within three minutes. |
| Callback does not match the current sign-in attempt | Use the browser tab opened by the current login command. |
| Microsoft denies access | Check the app account types, delegated permissions, and tenant consent policy. |
| HTTP 403 | Check Microsoft permissions and the services authorized at login. |

Use `auth export` and `auth import` to move credentials between machines.
Copying encrypted credential records alone is insufficient without their
original master key.
