# Security policy

## Supported versions

Only the latest release is supported. There are currently no published releases; the development branch is experimental.

## Reporting a vulnerability

Please do not disclose vulnerabilities in a public issue. Use [private vulnerability reporting](https://github.com/MohamadrezaKazemian/split-tunnel-v2ray/security/advisories/new) to send the maintainers reproduction steps, affected versions, impact, and any relevant sanitized logs. Maintainers will acknowledge and coordinate a fix as capacity allows; no response deadline is guaranteed.

The app starts privileged components and changes system traffic routing. A malicious engine or unsafe configuration can compromise the machine or expose traffic. Stored proxy credentials are plaintext in the application's local settings; protect those files and redact logs before sharing them.
