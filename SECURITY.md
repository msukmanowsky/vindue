# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 0.1.x   | yes       |

## Reporting a vulnerability

Please report vulnerabilities privately — **do not open a public issue**.

1. Preferred: use GitHub's **private vulnerability reporting** — Security tab
   → "Report a vulnerability" on this repository.
2. Once the report is received we will acknowledge within 7 days, work on a
   fix, and coordinate disclosure with you.

### Scope notes

Vindue's control API and MCP server bind to `127.0.0.1` only and reject
cross-origin requests (any request carrying `Origin`/`Sec-Fetch-Site`, or a
`Host` outside the loopback allowlist, gets `403`). This is deliberate: the
threat model treats local processes as trusted and browsers as the adversary.
If you find a way for a **web page or remote host** to reach the API, that is
in scope and important — please report it.

### CSP

The webview config sets `"csp": null` deliberately: the panel windows load
only locally bundled assets — no remote content, no user-supplied HTML — so a
Content-Security-Policy adds little defense today. It should be revisited if
the app ever loads remote content or renders untrusted input.
