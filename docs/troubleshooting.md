# Troubleshooting

Start with **Connection details** in the app. Separate local connection problems from token problems and MPass’s final authentication decision.

| Symptom                                    | First check                                                                     |
| ------------------------------------------ | ------------------------------------------------------------------------------- |
| **SafeNet is missing**                     | Install the Apple Silicon-compatible SafeNet driver; see driver paths below     |
| **Connection is paused** / port in use     | Quit MoldSign Server and duplicate app processes, then reconnect                |
| Repeated connection setup                  | Check the generated certificate/key pair and login-keychain trust               |
| No token certificate                       | Check the USB connection, SafeNet tools, then refresh the token list            |
| Browser says it cannot load certificates   | Check HTTPS trust, local-network permission, and whether the service is running |
| PIN error                                  | Do not guess or repeatedly retry; check the token’s remaining attempts          |
| Signature created, but MPass rejects login | Reload the login page, then investigate the signature/session boundary          |

## MPass cannot see the local app

Keep the app open. Verify that `localhost.cts.md` resolves to loopback and that the app reports a running service with verified trust. Finish certificate setup instead of bypassing a browser security warning.

A browser local-network permission is separate from HTTPS certificate trust. MPass may need both. Browser extensions can also interfere; inspect their behavior before changing settings globally.

Safari uses HTTP/2. The current service validates its URI authority as well as HTTP/1.1’s Host header. An older app build can return 403 even when the hostname is correct. Use the current bundle after rebuilding.

## Port 18443 is occupied

MoldSign Server and this app use the same local port. Quit the server and any duplicate copy of the bridge before reconnecting. A source build can coexist on disk with an older bundle; ensure the process you opened is the one you rebuilt.

The bridge does not change system services or kill other applications to claim the port.

## Certificate setup repeats or fails

The connection uses:

```text
~/Library/Application Support/dev.local.moldsignbridge.preview/connection/localhost.pem
~/Library/Application Support/dev.local.moldsignbridge.preview/connection/localhost-key.pem
```

Both files should remain together. Trust is checked again on each launch and cached briefly during use. Setup normally persists; it is not intended to run before every login.

If resetting a broken setup, first identify the generated certificate in Keychain Access by fingerprint. Remove only this app’s local connection certificate, quit the app, and reset its connection files together. Keep a private backup if you need rollback. Relaunch and complete setup again. Do not remove the token’s signing identity or unrelated trusted certificates.

The current file-creation path does not repair every partially missing key/certificate pair automatically; removing just one file can leave setup unable to regenerate the pair.

## Token or certificate is missing

Check that SafeNet’s own tools see the token. The driver must be accessible at one of:

```text
/Library/Frameworks/eToken.framework/Versions/A/libeToken.dylib
/usr/local/lib/libeToken.dylib
```

Reconnect the USB device and use **Refresh token**. Public certificate reads do not ask for a PIN. A connected token may hide certificate objects until login; that configuration is not covered by the verified read-only listing flow.

The browser listing omits certificates outside their validity dates and tokens reporting a locked PIN. Private-key availability is confirmed only during signing.

## Unsupported signing mechanism

A driver can advertise a mechanism without making it available for signing. The tested SafeNet token supports `CKM_RSA_PKCS` for signing and does not support the combined SHA-1 signing mechanism.

The current worker checks signing capability flags and builds the SHA-1 DigestInfo locally for the raw RSA path. A fallback is allowed only for mechanism rejection at `SignInit`; it never repeats the PIN login or retries after a signature operation has begun.

For a read-only capability check, run the bundle executable with `--token-capabilities`. Its output contains capability flags rather than certificate identities. Avoid posting `--probe-token` output, which contains public certificate metadata and can expose personal information.

## MPass says “Autentificarea nu a reușit”

This is a generic server-side authentication rejection. It does not identify whether the cause is the challenge/session, certificate policy or revocation, XML signature, timestamp, or another server check.

Use a freshly loaded MPass login page and complete that request promptly. Multiple old login tabs can make it harder to tell which session/challenge you are approving. Check whether the app produced a signature or reported its own error first.

During development, signatures that passed basic XMLDSig checks were still not compatible with the tested login flow. The complete working profile includes issuer-name order/escaping, object order, legacy canonicalization, Base64 wrapping, and the timestamp DER encoding marker. See [MPass protocol](mpass_protocol.md) before changing any of those details.

Run the independent verification script after changing signature construction. It verifies a disposable timestamp chain, not the live government service’s trust or the browser’s session. Do not infer successful authentication from “Signature created”; the browser’s MPass response is authoritative.

## A request is stuck

A request expires after two minutes. Cancel an unsigned request locally, then start again on the website. If a result was never retrieved, its operation can remain until expiry. There is no background PIN retry.

A probe worker is stopped after 20 seconds; a signing worker after 25 seconds. The timestamp HTTP request has a 15-second timeout. If the driver repeatedly stalls, inspect SafeNet independently before attempting further PIN operations.

[Documentation index](index.md)
