# Security model

The app bridges a browser login request to a hardware token. Its boundaries are the local website origin, the user’s approval, the SafeNet driver, and MPass’s verification of the result.

## What stays local

- The private signing key remains on the USB token.
- The PIN is entered in the local approval dialog, not a government webpage.
- The PIN is passed to an isolated worker through stdin, never a command-line argument or HTTP field.
- The app does not cache PINs, retry failed PIN attempts, or intentionally log PINs, challenges, certificates, or signature payloads.
- A new worker process is used for token operations. It logs out and exits after signing.

The worker checks PIN-lock and final-attempt flags. A final attempt requires explicit confirmation and is checked again before login. An incorrect PIN is returned as an error without an automatic retry.

Rust uses protected/zeroing buffers where practical. The PIN form is uncontrolled and keeps the value outside React state. JavaScript strings, serialization copies, OS memory, crash reports, and vendor-driver behavior cannot be guaranteed to be wiped. This is not a guarantee against a compromised Mac or a privileged local process.

## What leaves the Mac

| Destination                               | Data                                                                                |
| ----------------------------------------- | ----------------------------------------------------------------------------------- |
| MPass, through its browser flow           | Public certificate metadata and the completed login signature                       |
| Moldova timestamp service, after approval | Certificate serial in the URL, signature-value digest, nonce, and timestamp request |

The timestamp endpoint uses HTTP, following the inspected server configuration. The app checks response status, size, nonce, and imprint. It does not independently verify the live timestamp token’s signature/trust chain. See [MPass protocol](mpass_protocol.md) for the precise limit.

The app has no application telemetry. The browser, government websites, and SafeNet middleware have their own behavior and policies.

## Local HTTPS trust

Each installation generates its own certificate and private key for `localhost.cts.md`; the app does not reuse a shared MoldSign key. The directory is restricted to its owner, and the private key file is written with owner-only permissions.

Setup uses the user’s login keychain and restricts trust to the SSL policy and local hostname. It does not modify the system root keychain. Existing files and trust are reused across launches. A trust check is cached for up to 30 seconds.

Keep `localhost-key.pem` private. To remove trust, use Keychain Access and identify the generated certificate by its fingerprint. Never remove unrelated signing certificates or SafeNet identities as part of connection cleanup.

## Browser request boundary

The service binds only to IPv4 loopback. Guards enforce the configured host/HTTP2 authority and exact `https://mpass.gov.md` origin. There are no wildcard origins or credentialed CORS responses.

Requests are restricted to the observed MPass login shape. The selected certificate is resolved from a local cache, and a private key is located by `CKA_ID` after approval. Results are single-use, requests expire after two minutes, input/output sizes are bounded, and workers have deadlines.

Origin and Host checks protect the browser boundary. They are not authentication against another process running as the same Mac user: a native local program can construct those headers. Explicit approval is therefore still required for signing.

The app’s production content-security policy permits its local assets and Tauri IPC. The separate development policy allows Vite’s development scripts/styles and localhost hot-reload connection.

## Certificate and signature validation

The app checks certificate time validity and verifies the hardware-produced RSA signature against the selected certificate. It is not a complete local certificate trust, policy, or revocation validator. MPass must accept the signer’s certificate and the timestamp before granting login.

“Connected” means the local connection/driver checks passed. “Signature created” means a local signature was produced. Neither alone proves successful MPass authentication.

## Diagnostic commands

`--token-capabilities` reports signing capability flags without requesting a PIN. `--probe-token` prints certificate metadata, including DER data, to its caller; its raw output can contain personal certificate information and should not be posted publicly.

Never send a token PIN in chat, include it in a shell command, or add actual login payloads to repository fixtures. The independent interoperability script creates disposable keys and runs without a token.

[Documentation index](index.md)
