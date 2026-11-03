# Getting started

## Requirements

- An Apple Silicon Mac with Xcode Command Line Tools if you are building from source.
- A connected SafeNet USB token with a valid certificate and its PIN.
- SafeNet Authentication Client for macOS, including an Apple Silicon-compatible PKCS#11 driver.
- A browser that can access MPass and the local HTTPS connection.

The tested token is SafeNet ID Prime MD. The app discovers the driver at these locations, in order:

```text
/Library/Frameworks/eToken.framework/Versions/A/libeToken.dylib
/usr/local/lib/libeToken.dylib
```

SafeNet middleware remains necessary. MoldSign Server, MoldSign libraries, and Java are not needed to run the app. Build instructions are in [Development](development.md).

## First launch

Open **MoldSign Bridge Preview.app**. The app starts the local login service, checks certificate trust, and reads public certificates from the token. No PIN is requested for this initial check.

If the app shows **Set up connection**, choose it and complete the macOS prompt. Setup creates a unique local HTTPS certificate/key pair if one does not already exist, then asks macOS to trust the certificate in your login keychain for the SSL policy and hostname `localhost.cts.md`.

The certificate and key are reused on later launches. Trust is checked on startup, cached for 30 seconds, and refreshed after setup. You should not need to repeat setup every time.

Local connection files live at:

```text
~/Library/Application Support/dev.basicshapes.moldsign-bridge/connection/
  localhost.pem
  localhost-key.pem
```

On the first launch under the new identifier, a complete connection pair from the previous `dev.local.moldsignbridge.preview` app-data folder is copied automatically, provided the new connection folder does not already exist. The old pair is retained. This preserves the certificate fingerprint and its existing keychain trust.

Do not share `localhost-key.pem`. Do not bypass a browser certificate warning; use the app’s setup flow instead.

## Log in to MPass

1. Keep the app open and your token connected.
2. Open [MPass](https://mpass.gov.md/login) in your browser.
3. Choose USB-token login and select your certificate.
4. Review the website and certificate in the app’s approval dialog.
5. Enter your token PIN and choose **Approve login**.
6. Return to the browser for MPass’s authentication result.

The app restores its minimized window and temporarily stays above other windows while approval is pending. Both dialog buttons fill equal-width columns. **Cancel** and **Escape** cancel an unsigned request. A request expires after two minutes, including time spent waiting for approval.

A message that a signature was created means the local operation completed. MPass determines whether authentication succeeds.

## Everyday behavior

- The service runs while the app is open; it is not installed as a background system daemon.
- Use the refresh control to rescan a newly connected or removed token.
- Expand **Connection details** to inspect trust, driver, and service status.
- If the browser asks MPass for local-network access, allow it for this flow.
- Quit MoldSign Server before starting this app; both use port `18443`.

For errors, see [Troubleshooting](troubleshooting.md).

[Documentation index](index.md)
