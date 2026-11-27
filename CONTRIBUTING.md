# Contributing to Joom

## Report a bug or suggest a feature

Search existing issues before opening a new one. For bugs, include the Joom
version, operating system, a minimal command, expected behavior, and actual
behavior. Replace real account names, resource IDs, message content, and other
private data with examples. Never include OAuth secrets or tokens.

For feature requests, describe the task you want to complete and show an example
of the proposed command. Discuss changes to command syntax before implementing
a broad redesign.

Use [SECURITY.md](SECURITY.md) to report a vulnerability privately.

## Set up development

Install Rust 1.88 or newer, Cargo, and Make. Build with:

```sh
make build
```

Rust and Cargo are sufficient to build the native executable and its wrappers.
The maintainer tools use TypeScript. Install Node.js 22 or newer and run `npm ci`
before regenerating schemas, reference documentation, or running live-test tools.

## Keep command contracts consistent

Public command inputs live in `schemas/`. The pinned compatibility routing and
request contracts live in `api/`. The Microsoft adapter resolves supported
operations to Graph. Wrapper commands and their constraints are defined in
`schemas/wrappers.schema.json`.

The schema generator translates API inputs into CLI parameters. `x-cli-binding`
records how a parameter becomes a query value, body field, or local option.
JSON Schema assertions under `allOf` define requirements and input choices.
Help and runtime validation use those assertions after merging parameter files
with command-line values. Keep these rules in the schemas rather than copying
them into separate help or validation code.

When changing a command, update its parameter definition, validation, help,
examples, and documentation together. Preserve existing commands unless a
compatibility change has been agreed. Keep wrapper-only syntax out of `joom`.

After changing API contracts or schema generation, regenerate the public
schemas and command references:

```sh
make reference
```

## Validate changes

Run the relevant tests while developing, then run the checks used by CI:

```sh
make ci
npm run check:schemas
npm run check:wrappers
```

Add tests for behavior changes, regressions, and validation boundaries. Do not
put credentials, private API responses, or account-specific content in fixtures.

Normal tests do not require a Microsoft account. Explicit live checks use an
already signed-in account and never retry authentication. Read
[the testing guide](docs/testing.md) for account selection, fixture cleanup, and
recorded Microsoft coverage.

## Submit a pull request

Keep each pull request focused on one problem. Explain the resulting behavior,
any compatibility impact, and the checks you ran. Include examples when changing
commands or output. Update user documentation with the implementation.

Contributions submitted for inclusion in Joom are licensed under the
[Apache License, Version 2.0](LICENSE), unless explicitly stated otherwise.
Participation is subject to the [code of conduct](CODE_OF_CONDUCT.md).
