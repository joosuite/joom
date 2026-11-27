# Security policy

## Report a vulnerability

Do not put exploit details, credentials, or private account data in a public
issue or pull request.

Use **Security → Advisories → Report a vulnerability** on this repository when
GitHub private vulnerability reporting is available. If that option is absent,
use a private contact method published on the
[repository owner's profile](https://github.com/joko76ers). If no private
channel is listed, open an issue requesting one without disclosing the
vulnerability itself.

Include the affected version or commit, operating system, reproduction steps
using test data, expected and actual behavior, and the potential impact. Remove
tokens, client secrets, account content, and unrelated personal information.

## Fixes and disclosure

Security fixes target the current code on `main`. Older revisions do not have
a separate security maintenance commitment. Maintainers may ask for additional
reproduction details and coordinate disclosure with the reporter.

## Exposed credentials

If a token or credential has already been exposed, revoke the affected Microsoft
permissions or replace the credential through Microsoft before sharing diagnostic
information. Deleting local Joom tokens does not revoke Microsoft's authorization.

See [authentication and credential storage](docs/authentication.md) for Joom's
storage model and sign-out behavior.
