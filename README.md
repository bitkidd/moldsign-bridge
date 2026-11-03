# MoldSign Bridge Preview

Tauri 2 app for macOS Apple Silicon. Uses Rust and the independently installed SafeNet PKCS#11 driver. No MoldSign installation, Java runtime, or proprietary signing libraries are required.

## Try MPass login

1. Keep SafeNet Authentication Client installed and connect the token.
2. Quit MoldSign Server (it uses the same port).
3. Open the built app. The login connection starts automatically.
4. If setup is needed, choose **Set up connection**. macOS asks to trust this installation's unique local HTTPS certificate, restricted to the SSL policy for `localhost.cts.md`, in your login keychain. The private key stays in the app's data folder with owner-only permissions. No system root keychain changes are made.
5. Open MPass in your normal browser, choose the USB-token option, then your certificate.
6. The app restores from minimized state and opens a focused approval dialog above other windows. Enter your token PIN locally and choose **Approve login**. Return to the browser.

Certificate listing and local PIN approval have been verified. End-to-end MPass authentication needs confirmation after signature compatibility changes. Certificate trust is checked automatically at startup, cached for 30 seconds, and rechecked immediately after setup. Existing certificates and keys are reused across launches. Never enter a token PIN in chat. If the browser asks for local-network permission for MPass, allow it for this login flow. Don't bypass browser certificate warnings; finish local connection setup first.

## Implemented compatibility

Observed public MPass desktop page: `https://mpass.gov.md/moldsign/login?authenticationType=MoldSign`.

- HTTPS on IPv4 loopback only, `https://localhost.cts.md:18443`.
- Certificate listing: `GET /certificates?private_only=true`, JSON `certificateModel` array.
- `POST /sign/data`: SHA-1, Embedded, Text, XAdES-T, exactly 20 challenge bytes. Unexpected fields and other signature formats are rejected.
- Empty 201 response with exposed `Location` and `sessionId` headers, matching MPass's jQuery callback behavior.
- `GET /sign/data/PKCS11/{id}/XAdES` waits for local approval, then returns JSON `base64File`.
- Only exact `https://mpass.gov.md` Origin and the expected Host are accepted. Cross-origin preflight supports browser private-network checks. No HTTP listener, wildcard origins, remote clients, document-signing endpoints, or credentialed CORS.
- One request at a time, random identifiers, two-minute expiry, single-use results, 16 KiB HTTP body limit, bounded token subprocesses, explicit cancellation.
- PIN travels to an isolated signer over stdin, never command arguments or network. Rust buffers are cleared where practical; JavaScript/OS/driver copies cannot be guaranteed to be wiped. No PIN caching, automatic retry, logging, or telemetry. The PIN-attempt warning is checked again immediately before token login.
- Certificate key selected by CKA_ID, RSA PKCS#1 signature verified against the token certificate before delivery. SHA-1 is used only because this MPass page explicitly requests that legacy algorithm.
- XMLDSig with XAdES SigningTime/SigningCertificate and timestamp. Canonicalization uses the macOS libxml2 API through open-source Rust bindings.
- Timestamp service: the official MoldSign endpoint `http://tsp.pki.gov.md/moldsign2/{reversed-serial}`. It receives the certificate serial and SHA-256 digest of canonical SignatureValue after approval. The response nonce and imprint are checked locally; the relying party must validate TSA signature/trust. No claim of independent local TSA chain validation is made.

Public certificate metadata is sent only to MPass through its local requests. It is displayed locally and not persisted or logged by the app. `--probe-token` is a diagnostic command that emits certificate metadata to its caller; do not share its raw output.

## Build and checks

Requires Rust, Node/npm, Xcode command line tools, SafeNet Authentication Client for macOS. Runtime driver discovery uses `/Library/Frameworks/eToken.framework/Versions/A/libeToken.dylib`, then `/usr/local/lib/libeToken.dylib`. The driver is not bundled.

```sh
npm install
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
# Optional independent verifier; requires a JDK for this development check only:
./scripts/check_xml_signature.sh
```

App output: `src-tauri/target/release/bundle/macos/MoldSign Bridge Preview.app`.

Tests cover origin/host rejection, browser preflight, challenge validation, escaping, and malformed timestamp responses. XMLDSig interoperability was also checked using a disposable software certificate and Java's independent built-in XMLDSig verifier; Java is a development-only verifier, not an application dependency.

## Local connection reset

Use Keychain Access to remove the certificate matching this app's generated `localhost.pem` fingerprint if you stop using the app. Files are under the app data folder `~/Library/Application Support/dev.local.moldsignbridge.preview/connection`. Keep the certificate and private key together; deleting only one requires resetting both and running setup again. The listener runs only while the app is open.

## Desktop UI

Indigo/violet interface with a vector key mark and matching macOS icon. The default window is 480 × 680 logical pixels; allowed dimensions are 460–560 × 640–800. Fullscreen/maximize is disabled. Login approval uses a modal with keyboard focus, Escape to cancel, and PIN clearing on submit, cancellation, and expiry. The app is temporarily always on top while approval is pending, then returns to normal after signing, cancellation, or expiry.

The React frontend separates connection, token, and approval components, with a shared hook that prevents overlapping status requests. Vite builds the frontend into `dist/`. Developer diagnostics sit under Connection details. Phosphor supplies all interface icons and the app icon’s bold Key glyph. Run `npm run icons` to regenerate the SVG, PNG, and macOS ICNS assets. The Phosphor MIT notice is retained under `src-tauri/icons/`.

Run `npm run dev` for the Tauri app with Vite hot reload, `npm run dev:web` for the frontend development server, and `npm run build` for the production macOS app. The browser-only development server needs mocked Tauri commands to exercise token functionality. PIN entry stays in an uncontrolled input ref and is never kept in React state.

## Frontend structure and checks

React files use snake_case names and strict TypeScript. `components/` contains connection, token, layout, and login features; `hooks/` manages bridge state; `components/ui/` contains buttons, inputs, dialogs, cards, and notices; `screens/` composes views; `components/forms/` owns PIN form behavior; `lib/bridge_client.ts` types the Tauri calls; `types/` defines shared data contracts. PIN values stay in an uncontrolled input, outside React state.

`npm run check` runs TypeScript, Oxlint, and Oxfmt verification. `npm run format` formats supported source/config files with single quotes and sorted imports. Production frontend builds also run type checking.

Internal frontend imports use the `package.json` subpath mappings (`#components/*`, `#hooks/*`, `#ui/*`, and related folders). TypeScript’s Bundler resolution and Vite read these mappings directly; no duplicate aliases are needed in tsconfig or Vite. Oxlint rejects relative module imports to keep this consistent.

The XAdES interoperability check also verifies the certificate issuer as an X.500 name and the legacy object ordering (qualifying properties before the embedded challenge). Issuer names are formatted using RFC 4514 rather than the display-oriented certificate parser format. Tauri-generated schemas are excluded from Oxfmt because builds regenerate them.

## MoldSign server wire-format comparison

The installed `ClientCardServer-2.0.jar`, `PKCS11CardReader-2.0.jar`, and `TransferObjects-2.0.jar` were inspected locally to trace `/sign/data` through its XAdES worker and response model. The response is `base64File`, containing the UTF-8 signature XML encoded once as Base64. The input challenge is decoded once, embedded as Base64 in `ds:Object`, and referenced by `SignedInfo`. No proprietary libraries are linked or bundled.

The compatibility profile uses Canonical XML 1.0 with comments for SignedInfo, implicit Canonical XML 1.0 for references, Canonical XML 1.0 for SignatureTimeStamp, 64-column Base64 for signature/certificate/timestamp values, and the explicit XAdES DER timestamp encoding marker. Existing issuer-name and object-order checks remain in place.

`./scripts/check_xml_signature.sh` now creates disposable signer and timestamp certificates, validates the generated XML using Java's built-in XMLDSig verifier, then independently verifies the timestamp signature, imprint, and test trust chain using macOS OpenSSL and xmllint. This local test does not establish MPass session or certificate-policy acceptance. It never accesses the USB token or requests a PIN.
