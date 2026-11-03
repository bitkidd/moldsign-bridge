<div align="center">
  <img src="ui/public/key.svg" width="88" height="88" alt="MoldSign Bridge key icon" />
  <h1>MoldSign Bridge</h1>
  <p>A small macOS app that connects your SafeNet token to MPass.</p>
  <p><strong>Tauri · Rust · React · TypeScript · Vite · Phosphor</strong></p>
  <p><a href="docs/getting_started.md">Get started</a> · <a href="docs/architecture.md">How it works</a> · <a href="docs/development.md">Development</a> · <a href="docs/troubleshooting.md">Troubleshooting</a></p>
</div>

---

## Your token. A simpler connection.

Open the app, choose your certificate on MPass, and approve the login in a focused PIN dialog. The connection starts automatically, certificate setup is remembered, and your PIN stays on your Mac.

The app communicates directly with the installed **SafeNet Authentication Client** through PKCS#11. It runs without MoldSign libraries, MoldSign Server, or a Java runtime.

> **Preview status:** End-to-end MPass USB-token login was confirmed on November 3, 2026, using an Apple Silicon Mac and a SafeNet ID Prime MD token. Other tokens, government websites, and document-signing workflows have not been validated.

## Start using it

1. Install SafeNet Authentication Client for macOS and connect your token.
2. Quit MoldSign Server if it is running; both apps use the same local port.
3. Open **MoldSign Bridge Preview.app**. Complete **Set up connection** if prompted. This is normally a one-time step on this Mac.
4. Visit [MPass](https://mpass.gov.md/login), choose USB-token login, and select your certificate.
5. Enter the token PIN in the app and choose **Approve login**. Continue in your browser.

The approval dialog brings the app forward, including from a minimized state. **Cancel** or **Escape** dismisses the request. The app returns to its normal window level when the request finishes.

See [Getting started](docs/getting_started.md) for certificate setup and connection details.

## Build locally

You need macOS, Xcode Command Line Tools, Rust, Node.js/npm, and an Apple Silicon-compatible SafeNet driver.

```sh
npm ci
npm run dev
```

Create the macOS app:

```sh
npm run build
```

The bundle is generated at:

```text
src-tauri/target/release/bundle/macos/MoldSign Bridge Preview.app
```

## Inside the app

| Layer                  | Responsibility                                                     |
| ---------------------- | ------------------------------------------------------------------ |
| React + TypeScript     | Connection status, certificate cards, and local PIN approval       |
| Tauri + Rust           | Local HTTPS service, request validation, and process orchestration |
| SafeNet PKCS#11 driver | Public certificate access and signing with the token’s private key |
| MPass                  | Login challenge, signature verification, and authentication result |

The localhost service accepts only the configured MPass origin. Token operations run in bounded child processes. The private signing key remains on the token; the app neither exports nor bundles it.

Read the [security model and its limits](docs/security.md), including PIN handling and timestamp verification.

## Project map

```text
ui/src/
  components/         Connection, token, layout, and login components
    forms/            Local PIN form
    ui/               Buttons, inputs, dialogs, cards, and notices
  hooks/              Bridge state and request polling
  screens/            Screen composition
  lib/                Typed Tauri client
  types/              Shared frontend contracts
src-tauri/src/        HTTPS bridge, token worker, and XAdES signing
scripts/              Icon generation and independent signature verification
docs/                 Setup, architecture, protocol, security, and development
```

Frontend filenames use **snake_case**. Internal imports use `package.json` mappings such as `#components/…`, `#forms/…`, and `#ui/…`. Oxlint and Oxfmt enforce the code conventions.

## Check a change

```sh
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
```

For independent XML signature and timestamp verification:

```sh
./scripts/check_xml_signature.sh
```

This optional development check requires a JDK, Python 3, and macOS OpenSSL/xmllint. It creates disposable test keys and certificates; it does not use your token or ask for a PIN. Java is **not** an app dependency.

## Documentation

- [Getting started](docs/getting_started.md) — requirements, setup, and everyday login.
- [Architecture](docs/architecture.md) — application layers and the login sequence.
- [MPass protocol](docs/mpass_protocol.md) — endpoints, payloads, and XAdES compatibility.
- [Security](docs/security.md) — trust boundaries, PIN handling, and current limits.
- [Development](docs/development.md) — structure, commands, conventions, and verification.
- [Troubleshooting](docs/troubleshooting.md) — connection, token, and login failures.

This is a local compatibility project. It is not an official MoldSign, MPass, or SafeNet release. Phosphor’s icon license is retained in `src-tauri/icons/PHOSPHOR-LICENSE.txt`.
