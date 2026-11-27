# Tests

Run the local suite with:

```sh
npm ci
make test
make lint
make fmt-check
```

The TypeScript tooling requires Node.js 22 or newer. `make test` runs the Rust
tests and the offline TypeScript tests. `make tooling-check` checks TypeScript
types and confirms that the generated schemas and wrapper reference are current.

Local contract tests cover 77 direct commands across six service groups and
53 commands across eight wrappers. They check request translation,
required inputs, incompatible parameters, JSON types, file overrides, output
parity, MIME handling, pagination, URL validation, and OAuth callbacks.
Adversarial cases use local parsers and loopback HTTP servers. They do not send
malformed requests to Microsoft or use account credentials.

All test code lives in `tests/`. Rust unit modules are in `tests/unit/` and are
included by their source modules only for test builds so they can check private
implementation details. Process-level checks are the Rust files directly under
`tests/`. The live runner, fixture helper, coverage reporter, and their offline
checks are in `tests/live/`.

Microsoft route fixtures check every exposed direct command and rejection of
removed commands and fields. Wrapper checks pass representative inputs through the
Microsoft adapter. They do not count as successful live Microsoft executions.

Credential tests cover encrypted export/import, account replacement, headless
storage, and listing accounts without unlocking credentials. They use synthetic
credentials in temporary vaults.

## Native credential storage

CI checks Windows Credential Manager and an isolated Linux Secret Service. These
tests create disposable keys and encrypted records, open a legacy vault,
reopen the store, move the vault, and check that the original key and encrypted
bytes are preserved. Both temporary credential-store entries are deleted.

Run these checks against an available OS credential store with:

```sh
cargo test --locked store::tests::real_ -- --ignored --nocapture
```

Linux needs an unlocked Secret Service on the user's D-Bus session. CI starts a
temporary keyring for this purpose. These tests do not use Microsoft credentials.

## Live account checks

Live tests require an explicitly selected account that is already signed in:

```sh
JOOM_LIVE_ACCOUNT=test-account@example.com make live-test
```

For two approved accounts:

```sh
JOOM_LIVE_ACCOUNTS=first@example.com,second@example.com make live-test
```

Select services with `JOOM_LIVE_SERVICES=mail,drive`. Focused cases use
`JOOM_LIVE_CASE=mail-draft`, `drive-metadata`, `drive-files`, `office-files`, or `regressions`
with the corresponding service selection. These avoid repeating unrelated
successful scenarios. The runner builds a test-only
binary with `live-test-diagnostics`, copies it to `target/live-test-bin`, and
holds a run lock. Normal tests never run these Microsoft checks. The live runner
requires Node.js 22 or newer and `npm ci`, as well as Rust and Cargo.

`JOOM_LIVE_SESSION_ONLY=1` also disables token refresh and HTTP retries in normal
builds. Diagnostic tracing and fixture lock bypass still require the test feature.

The runner uses existing access tokens. It never opens sign-in, enrolls a client,
or refreshes a token. An expired token stops the account before an authentication
request. Authentication failures, throttling, server errors, and timeouts stop
that account. They also leave any remaining cleanup recorded. There are no
command or HTTP retries. Commands run sequentially with a one-second pause;
each has a 15-second HTTP timeout and a 40-second process deadline. Each account
has a 300-command budget, with 40 additional commands reserved for cleanup.

Each account is checked through its Microsoft profile before fixture creation.
Tests use a unique `joom-test-` marker and create synthetic messages, mail
folders, OneDrive files, secondary calendars without attendees, To Do lists,
and contacts. A native fixture helper creates a two-occurrence event in the
synthetic calendar so the CLI can test recurring-event instances. That setup
does not count as command coverage. Mail goes to the account's own mailbox
address, verified through Graph when the login address uses Gmail. Older reports
can record external Gmail deliveries for manual cleanup. Graph cannot remove
those external copies.

Sharing uses synthetic files, disables email notifications, and uses only the
other explicitly approved account. OneDrive fixture cleanup verifies the marker
in each registered file's name, then uses Microsoft's documented
[`bypass-shared-lock`](https://learn.microsoft.com/en-us/graph/api/driveitem-delete?view=graph-rest-1.0)
header to recycle it. This behavior exists only in the diagnostic build and
requires session-only mode and the current fixture marker. Cleanup never empties
the recycle bin or changes existing files. Word,
Excel, and PowerPoint fixtures are checked-in Office files uploaded through
Graph. Excel content checks require a storage/account combination supported by
Microsoft's workbook API; unavailable operations remain missing coverage.

The runner records fixture cleanup immediately after receiving a creation ID.
An unexpected service failure stops its dependent checks. Reports exclude
tokens and retrieved mail/document contents. They include command parameters
for synthetic fixtures, Graph error codes, binary fingerprints, per-command
coverage, and pending cleanup IDs.

Reports are private JSON files under `target/microsoft-live/`. Each run has a
separate filename. A full run returns failure when coverage is incomplete;
missing or rejected operations never count as successful tests.

Recover only registered fixtures after resolving a failure:

```sh
JOOM_LIVE_ACCOUNT=test-account@example.com \
  JOOM_LIVE_CLEANUP_REPORT=target/microsoft-live/report-ACCOUNT-RUN.json make live-cleanup
```

Cleanup makes one attempt per resource, accepts HTTP 404 as already absent, and
stops on unexpected failure. Locked files stay recorded while other registered
fixtures each get one cleanup attempt. It uses the existing session and never signs in or
refreshes. Keep any report with pending cleanup until those fixtures have been
removed. Self-addressed mail gets one bounded pass through global, inbox, and
sent searches, with at most 20 results per collection. The runner filters exact
fixture subject prefixes and deduplicates IDs. It never polls or resends mail.

Combine recorded coverage without making Microsoft requests:

```sh
make live-coverage
```

Set `JOOM_LIVE_REPORTS` to a platform-delimited list of reports, `:` on Unix or `;` on Windows, to select
particular runs. The output is `target/microsoft-live/coverage.json`. It retains
historical failures, source reports, binary fingerprints, and cleanup entries.
`completeRuns` identifies full runs that passed all commands on one binary and
finished cleanup. Historical failures remain in the report after a fix passes.
Successful calls across different runs do not certify that one current binary
passed every command.

The runner's offline parsing and stop-condition tests use no accounts:

```sh
npm test
```
