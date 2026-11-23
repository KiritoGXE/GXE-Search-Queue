# Security Policy

## Reporting a vulnerability

We take security seriously. If you believe you have found a security
vulnerability in GXE-Search-Queue, please report it to us privately before
creating a public issue.

**Do not open a public GitHub issue** for security problems.

### How to report

Send a private report to the maintainers via GitHub's
[Security Advisories](https://github.com/) ("Report a vulnerability" button on
the repository's *Security* tab), or email the maintainers directly if a
contact is listed.

Please include:

- A description of the vulnerability and its impact
- Steps to reproduce (code snippets, configs, payloads)
- Affected versions and components
- Any suggested mitigation, if you have one

### What happens next

1. We confirm receipt within **72 hours**.
2. We assess severity and impact.
3. We work on a fix and coordinate a release.
4. We publish a security advisory once the fix is released.

We ask that you allow us a reasonable disclosure window before public
publication.

## Supported versions

Security fixes are provided for the latest minor release of the current major
version. Older versions are supported on a best-effort basis.

## Security best practices

- Always run with the least privilege required.
- Rotate secrets stored in `.env` / environment variables.
- Keep dependencies up to date; CI will fail on known-vulnerable dependencies.
- Review network exposure of the REST API and dashboard.
