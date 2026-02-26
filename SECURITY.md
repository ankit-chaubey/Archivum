# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.2.x   | ✅ |
| 0.1.x   | ✗ (end of life) |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Email: [ankitchaubey.dev@gmail.com](mailto:ankitchaubey.dev@gmail.com)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

You will receive a response within **72 hours**. If confirmed, a patch release will be issued within **7 days**.

## Security Design

- **Blake3 index seal**: Any tampering with `index.arc.json` is detected before restore or verify
- **SHA-256 per-file checksums**: Every restored file is verified against the stored hash
- **Path traversal guard**: Archive entries with `..` path components are rejected during restore
- **No network access**: Archivum operates entirely offline — no telemetry, no remote calls
