# Security Policy

## Supported Versions

| Version | Supported          |
|:--------|:-------------------|
| 1.x     | Yes                |
| < 1.0   | No                 |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Instead, please email **security@nyzhi.com** with:

- A description of the vulnerability
- Steps to reproduce or a proof-of-concept
- The affected version(s)
- Any potential impact assessment

You should receive an acknowledgment within **48 hours**. We will work with you
to understand the issue and coordinate a fix and disclosure timeline.

## What to Expect

1. **Acknowledgment** within 48 hours of your report.
2. **Assessment** — we will confirm the issue, determine severity, and identify
   affected versions.
3. **Fix** — a patch will be developed and tested privately.
4. **Release** — a new version will be published with the fix.
5. **Disclosure** — a security advisory will be published on GitHub after the
   fix is available.

We aim to resolve critical vulnerabilities within **7 days** of confirmation.

## Scope

The following are in scope:

- The `nyz` binary and all workspace crates
- The install script (`get.nyzhi.com`)
- Token storage and credential handling
- The self-update mechanism and binary verification

The following are **out of scope**:

- Third-party LLM provider APIs
- Issues in upstream dependencies (report those to the upstream project)
- Social engineering attacks

## Credit

We are happy to credit reporters in the security advisory unless you prefer to
remain anonymous. Let us know your preference when reporting.
