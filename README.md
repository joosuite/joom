# <img src="assets/brand/logo.png" alt="Joom" width="240" height="120">

Microsoft services from the terminal, with named command options, JSON parameter
files, and short commands through native wrappers.

Joom uses Outlook Mail, OneDrive, Outlook Calendar, Excel, Word files, PowerPoint
files, Microsoft To Do, and Outlook Contacts. See [Microsoft support](docs/microsoft.md)
for implemented operations and provider limits.

Joom is an independent open-source project and is not affiliated with, endorsed
by, or sponsored by Microsoft. It is provided "as is," without warranties of any
kind or guarantees of availability, reliability, or continued access to Microsoft
services.

## Build and install

Building from source requires Rust 1.88 or newer and Cargo. With Make installed:

```sh
make build
./bin/joom --help
```

The build creates `bin/joom` and eight wrapper entry points. To install them
in Cargo's binary directory:

```sh
make install
```

On Windows, Cargo can build Joom and create the wrappers without Make:

```powershell
cargo build --release --locked
cargo run --release --locked --example install_wrappers -- --directory target/release
```

Joom and its wrappers are native Rust executables. Node.js 22 or newer and
`npm ci` are needed only for the TypeScript maintainer and live-test tooling.

## Sign in

Register a mobile and desktop application in Microsoft Entra, enable the account
types you need, and register the loopback callback. Follow the
[sign-in setup](docs/authentication.md) for Microsoft registration and permissions.

```sh
joom auth init
joom auth enroll
joom auth you@example.com all
```

`all` requests the configured permissions for every supported service. To choose
services individually:

```sh
joom auth you@example.com login mail,drive,calendar
```

Use `joom auth list` to list registered accounts, or `joom --json auth list` for
a JSON array.

Account tokens and client credentials are encrypted locally. Joom stores the
master key in the operating system's credential store by default: macOS Keychain,
Windows Credential Manager, or Linux Secret Service.

For a VPS without a browser, sign in on your laptop and transfer an encrypted
credential file:

```sh
# Laptop
joom auth you@example.com export credentials.joom
scp credentials.joom user@vps:~/

# VPS
joom auth import credentials.joom --headless
```

Both commands prompt for the transfer passphrase. The headless vault uses a
private local key file and works without a desktop keychain. See the
[VPS setup guide](docs/authentication.md#use-joom-on-a-vps) for account selection,
replacing credentials, and automation.

## Use Joom

```sh
joom mail you@example.com messages.list --q is:unread --max 10
joom drive you@example.com files.list --max 10
joom --json drive you@example.com files.list --max 10
joom mail you@example.com messages.send --help
joom mail you@example.com messages.send --help --verbose
```

Common parameters appear in `--help`; `--help --verbose` includes advanced
parameters. Use `--file PATH` to supply command parameters from a JSON object.

The wrappers provide shorter commands and `key:value` parameters:

```sh
jmmail you@example.com inbox limit:10 unread:true
jmcal you@example.com today
jmdrive you@example.com ls limit:10
```

Short syntax belongs to the wrappers. The corresponding `joom` commands remain
available. A wrapper's `json:true` option changes the output format while
preserving its result data.

## Documentation

- [Command reference](docs/grammar.md)
- [Short commands and wrappers](docs/wrappers.md)
- [JSON parameter files and schemas](docs/file-schemas.md)
- [Request examples and their commands](examples/requests/README.md)
- [Authentication and credential storage](docs/authentication.md)
- [Local and live tests](docs/testing.md)

## Contribute

Read [CONTRIBUTING.md](CONTRIBUTING.md) for development checks and pull requests,
and the [code of conduct](CODE_OF_CONDUCT.md) for participation guidelines.
For vulnerabilities, follow [SECURITY.md](SECURITY.md).

## License

Joom is licensed under the [Apache License, Version 2.0](LICENSE).
Third-party dependencies retain their respective licenses.
