# Development

## Toolchain

The app uses Tauri 2/Rust for macOS integration and a React/TypeScript frontend built with Vite. Phosphor supplies interface icons and the Key glyph used in the app icon. Runtime signing is through the independently installed SafeNet driver.

Install Xcode Command Line Tools, Rust, and a current Node.js/npm toolchain compatible with the lockfile. Development is validated on Apple Silicon. The Rust dependency versions are pinned in `src-tauri/Cargo.lock`; npm versions are pinned in `package-lock.json`.

```sh
npm ci
npm run dev
```

Tauri starts Vite on `http://localhost:1420`, then opens the desktop app. `npm run dev:web` starts only Vite; a normal browser has no Tauri IPC, so token functionality requires mocks or the desktop runtime.

## Commands

| Command                                           | Purpose                                      |
| ------------------------------------------------- | -------------------------------------------- |
| `npm run dev`                                     | Desktop app with Vite hot reload             |
| `npm run dev:web`                                 | Frontend development server only             |
| `npm run build:web`                               | Type-check and build `dist/`                 |
| `npm run build`                                   | Build the macOS `.app` bundle                |
| `npm run typecheck`                               | Strict TypeScript verification               |
| `npm run lint`                                    | Oxlint with warnings treated as failures     |
| `npm run format`                                  | Oxfmt formatting and import sorting          |
| `npm run format:check`                            | Verify formatting without writing            |
| `npm run check`                                   | TypeScript, lint, and formatting checks      |
| `npm run icons`                                   | Generate SVG, PNG, and macOS ICNS icons      |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust protocol and signing tests              |
| `./scripts/check_xml_signature.sh`                | Independent XML signature and timestamp test |

The app bundle is written to `src-tauri/target/release/bundle/macos/MoldSign Bridge Alpha.app`. Build configuration and window limits are in `src-tauri/tauri.conf.json`.

## Source map

```text
ui/src/
  app.tsx                         App-level icon context
  main.tsx                        React entry point
  style.css                       Shared visual styles
  components/
    connection/                   Connection status and details
    forms/login_form.tsx          PIN form, confirmation, and submission
    layout/                       Header, footer, and app mark
    login/login_dialog.tsx        Approval-dialog composition
    token/                        Certificate cards and token list
    ui/                           Reusable button/input/dialog/card/notice
  hooks/use_bridge.ts             Initialization, polling, and actions
  screens/home_screen.tsx         Screen composition
  lib/bridge_client.ts            Typed Tauri command calls
  types/bridge.ts                 Frontend data contracts
src-tauri/src/
  main.rs                         App entry, driver discovery, probe worker
  bridge.rs                       Local HTTPS and approval lifecycle
  connection_migration.rs         Legacy local HTTPS identity migration
  token_sign.rs                   PKCS#11 signer and mechanism selection
  xades.rs                        XML envelope and timestamp handling
scripts/
  generate_icons.mjs              Phosphor-based icon generation
  check_xml_signature.sh          Disposable interoperability fixture
  interop/                        Independent development verifiers
```

## Conventions

- Use snake_case filenames for frontend code. React components remain PascalCase; hooks use React’s `use…` naming convention.
- Put reusable primitives in `components/ui`, form behavior in `components/forms`, and feature components in their destination folder.
- Compose views in `screens`; keep bridge orchestration in hooks and IPC calls in the typed client.
- Use strict TypeScript and shared contracts; avoid untyped escape hatches.
- Use single quotes, including JSX attributes, and sorted imports. Run Oxfmt instead of sorting by hand.
- Keep PINs outside React state, logs, URLs, and persistent storage.

## Package imports

`package.json` is the single source of truth for internal subpath imports:

```tsx
import { LoginForm } from '#forms/login_form';
import { useBridge } from '#hooks/use_bridge';
import { Button } from '#ui/button';
```

TypeScript’s Bundler resolution and Vite resolve these mappings directly. Do not duplicate them with `paths` or Vite aliases. Oxlint rejects relative module imports. Both `#forms/*` and `#ui/*` point inside `components/`.

## Verification strategy

Run checks appropriate to the change. A frontend-only edit needs `npm run check` and a production frontend build. Changes to approval or focus behavior also need a desktop check. Do not enter a real PIN for layout/cancellation tests.

Rust tests cover origin/host rejection, HTTP/2 authority handling, preflight, challenge constraints, signing method selection, no retry after signing/authentication errors, issuer-name formatting, escaping, and malformed timestamps.

The independent script requires a JDK and Python 3 in addition to macOS OpenSSL/xmllint. It creates a disposable software signer and a separate timestamp certificate chain, exercises the XAdES builder and timestamp-response parser, verifies XMLDSig and compatibility metadata using Java, then verifies the timestamp using OpenSSL and independently canonicalized XML. Its temporary keys are removed on exit. The optional Rust fixture is activated by the script’s environment variables; a plain `cargo test` does not run that external verification.

Local tests establish construction and interoperability properties. Only an actual MPass login confirms that a current browser session, certificate, and server policy accept the result. The user confirmed that full flow on November 3, 2026.

## Generated assets

`dist/`, `node_modules/`, Rust `target/`, and Tauri `gen/` schemas are generated outputs. Oxfmt excludes generated/vendor assets and lockfile formatting. Do not hand-edit built JavaScript or generated Tauri schemas.

`npm run icons` renders the official Phosphor bold Key on the violet background and regenerates the macOS icon sizes. Preserve the retained Phosphor license. No image-generation service is needed for these vector assets.

[Documentation index](index.md)
