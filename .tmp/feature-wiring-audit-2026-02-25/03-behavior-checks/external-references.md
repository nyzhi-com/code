# External Reference Baseline (Exa + Tavily)

These references were collected to resolve behavior expectations where repo docs/tests were incomplete.

## Command UX for missing required arguments

Consensus baseline: commands with missing required args should emit a clear usage/error message and return non-zero status (rather than silently falling through as normal input).

- GNU coding standards and error handling guidance:
  - https://www.gnu.org/prep/standards/standards.html
  - https://www.gnu.org/prep/standards/html_node/Errors.html
- POSIX/Bash exit-status convention for usage errors:
  - https://www.gnu.org/s/bash/manual/html_node/Exit-Status.html
- Modern CLI style guidance:
  - https://clig.dev/
  - https://learn.microsoft.com/en-us/dotnet/standard/commandline/design-guidance

## Security guardrail behavior (policy/hook evaluation)

Consensus baseline: security and authorization checks should fail closed by default; placeholder success responses are unsafe.

- OWASP fail-secure principle:
  - https://www.cgisecurity.com/owasp/html/ch04s02.html
- Fail-open vs fail-closed security discussion:
  - https://authzed.com/blog/fail-open
  - https://community.cisco.com/t5/security-knowledge-base/fail-open-amp-fail-close-explanation/ta-p/5012930

## Configuration drift and dead config keys

Consensus baseline: config keys should be actively wired or removed/deprecated; silent drift increases operational risk and troubleshooting cost.

- Configuration drift best-practice discussions:
  - https://www.reach.security/blog/what-is-configuration-drift-5-best-practices-for-your-teams-security-posture
  - https://www.josys.com/article/article-saas-security-5-best-practices-to-identify-and-resolve-configuration-drift-in-your-it-systems
  - https://thenewstack.io/the-engineers-guide-to-controlling-configuration-drift/
