# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| < latest| :x:                |

## Reporting a Vulnerability

We take the security of Alex's Tube V seriously. If you believe you have found a security vulnerability, please report it to us as described below.

**Please do NOT report security vulnerabilities through public GitHub issues.**

### How to Report

You can report a security vulnerability through one of the following channels:

1. **GitHub Private Vulnerability Reporting** (preferred): Use the [GitHub Security Advisory tab](../../security/advisories/new) to privately report a vulnerability.

2. **GitHub Issue**: If private reporting is unavailable, create an issue with the label `security` and include `[SECURITY]` in the title.

### What to Include

Please include the following information in your report:

- Type of vulnerability (e.g., buffer overflow, SQL injection, cross-site scripting)
- Full path(s) of the affected source file(s)
- Step-by-step instructions to reproduce the vulnerability
- Proof-of-concept or exploit code (if available)
- Impact of the vulnerability (what an attacker could achieve)
- Any potential remediation steps you've identified

### Response Timeline

- **Acknowledgement**: We will acknowledge receipt of your vulnerability report within **48 hours**.
- **Assessment**: We will provide a detailed assessment within **7 days**.
- **Resolution**: We aim to resolve critical vulnerabilities within **30 days**.

### Disclosure Policy

- We follow **coordinated disclosure** practices.
- We will work with you to understand and validate the vulnerability.
- A fix will be developed and tested before public disclosure.
- Public disclosure will be coordinated with the reporter.
- Credit will be given to the reporter (unless anonymity is requested).

### Security Architecture

Alex's Tube V implements multiple layers of security:

- **WebView CSP**: Content Security Policy blocks external script injection
- **Axum hardening**: Request body size limits (10MB), ephemeral port binding, CORS restrictions
- **Input validation**: Integer overflow guards, path traversal prevention, log injection sanitization
- **Error scrubbing**: Opaque error messages prevent information disclosure
- **Dependency auditing**: Dependabot automated monitoring for vulnerable dependencies
- **Secret scanning**: GitHub secret scanning enabled

## Security Best Practices for Contributors

1. Never commit secrets, tokens, or credentials to the repository
2. Use `cargo audit` to check dependencies for known vulnerabilities
3. Run `cargo clippy` before submitting pull requests
4. Follow the principle of least privilege for all system access
5. Validate and sanitize all user input at trust boundaries
