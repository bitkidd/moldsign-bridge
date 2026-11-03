# Alpha releases and updates

The app is **MoldSign Bridge Alpha**, version `0.1.0-alpha.1`. Its permanent identifier is `dev.basicshapes.moldsign-bridge`, so moving between numbered Alpha releases preserves connection setup.

## Builds versus releases

- **Build macOS Alpha** runs on main pushes, pull requests, and manual requests. It checks TypeScript, lint, formatting, Rust tests, and the independent XML signature verifier; then uploads an Apple Silicon app ZIP. This is an ad-hoc signed development build, without notarization or update signing. Pull requests never receive release credentials.
- **Release macOS Alpha** runs for tags matching `v*-alpha.*`. It requires the `release` GitHub environment, validates matching versions, builds an Apple Silicon app and DMG, signs with Developer ID, submits for notarization, and creates signed updater archives. It creates a **draft prerelease**, which you review before publishing. A final check verifies the app signature, stapled notarization ticket, Gatekeeper assessment, and system-library dependencies. If any step fails, do not publish the draft.
- **Publish Alpha update channel** runs when a numbered Alpha prerelease is published. It copies its `latest.json` to the `alpha` channel release. This is necessary because GitHub’s “latest release” shortcut excludes prereleases. The manifest retains URLs to the immutable numbered release assets. Older versions cannot move the channel backwards.

The workflows use `macos-15`, an Apple Silicon runner. Intel builds are intentionally absent: the current tested hardware and SafeNet driver are ARM64. GitHub-hosted runners cannot validate the real USB token; perform a local login smoke test before publishing.

## Signing credentials

Three unrelated identities have different jobs:

| Identity                             | Purpose                                       | Where it lives                               |
| ------------------------------------ | --------------------------------------------- | -------------------------------------------- |
| Developer ID Application certificate | Identifies the app publisher to macOS         | Apple account and GitHub environment secrets |
| Tauri update signing key             | Proves an update was produced by this project | Offline backup and GitHub environment secret |
| Local connection certificate         | Secures browser requests on this Mac          | Per-user app data and local keychain trust   |

A paid Apple Developer membership and a **Developer ID Application** certificate with its private key are needed for notarized distribution outside the App Store. Export the certificate and private key together as a password-protected `.p12`. Create an App Store Connect team API key for notarization and download its `.p8` private key. See [Tauri’s macOS signing guide](https://v2.tauri.app/distribute/sign/macos/) for issuance and required API permissions.

Create a GitHub environment named `release`, restrict deployment to the Alpha release tags, and configure these secrets. Required reviewers can provide an additional publication checkpoint:

| Secret                               | Value                                                                   |
| ------------------------------------ | ----------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`                  | Base64-encoded exported `.p12` file                                     |
| `APPLE_CERTIFICATE_PASSWORD`         | Password used when exporting the `.p12`                                 |
| `APPLE_SIGNING_IDENTITY`             | Full `Developer ID Application: … (TEAMID)` identity                    |
| `APPLE_API_ISSUER`                   | App Store Connect API issuer ID                                         |
| `APPLE_API_KEY`                      | API key ID                                                              |
| `APPLE_API_PRIVATE_KEY`              | Full `.p8` file contents, including PEM markers                         |
| `TAURI_SIGNING_PRIVATE_KEY`          | Full contents of the local `.secrets/updater.key` file                  |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Leave unset for the initially generated key; otherwise use its password |

`GITHUB_TOKEN` is supplied by Actions. Apple signing credentials are imported into a temporary runner keychain and removed at job completion. The `.p8` and `.p12` files are private temporary files, also removed at completion. Secrets are never written into the app or committed.

The initial Tauri key pair was generated locally under `.secrets/`, which is ignored by Git. Back up `updater.key` in a secure password manager or encrypted offline storage **before publishing the first release**. Only its public counterpart is embedded in the app. Losing or replacing this key breaks updates for installed versions trusting the original key. Do not generate a new key for each release. See [Tauri’s updater guide](https://v2.tauri.app/plugin/updater/).

## SafeNet and the hardened runtime

The app enables the hardened runtime. `src-tauri/Entitlements.plist` grants `com.apple.security.cs.disable-library-validation` because the SafeNet PKCS#11 module is installed and signed by a different developer. Without this exception, a signed release can fail to load the otherwise working driver. This is the specific exception Apple documents for [third-party signed plug-ins](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.disable-library-validation). The app does not bundle SafeNet or MoldSign libraries.

The CI build explicitly links libxml2 from the macOS SDK and checks that the resulting executable has no absolute dependencies on Homebrew or a runner’s home directory.

## Public distribution

The source repository `bitkidd/moldsign-bridge` is public. This is also the release host configured in `src-tauri/tauri.conf.json`. The app uses anonymous HTTPS downloads; no GitHub token is embedded. Keep this repository and its numbered release assets public for installed apps to receive updates. Moving distribution to a different host requires a transition release while the old endpoint remains available.

## Update behavior

Open **Connection details → Check for updates**. The app checks only the configured HTTPS Alpha channel and offers **Install & restart** when a newer version exists. Nothing downloads or installs automatically. The native backend verifies the update signature before installation. Failed downloads or signature verification leave the installed version unchanged.

Installation is disabled during a pending login. The backend reserves the login service while downloading/installing and rejects new login requests until completion. On failure it releases that reservation; on success the app restarts. Finish any browser login before updating. Install the app in Applications before using the updater; running from a read-only disk image is unsuitable.

Until the first release/channel is published, the check reports that the Alpha release channel is unavailable. Ordinary local builds do not need a signing key; updater artifacts are enabled only in the release workflow.

## Release checklist

1. Update `version` together in `package.json`, `package-lock.json` (including its root package), `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`. Use an increasing numbered version such as `0.1.0-alpha.2`.
2. Run `node scripts/check_release.mjs`, `npm run check`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `cargo test --locked --manifest-path src-tauri/Cargo.toml`, and `sh scripts/check_xml_signature.sh`.
3. Commit the changes conventionally, push them, and push the matching tag (for example `v0.1.0-alpha.2`). The workflow rejects a tag/version mismatch.
4. Wait for the complete release workflow to succeed. Inspect the draft’s DMG, app archive, signature, and `latest.json`. Test the signed downloaded app on a Mac with SafeNet installed, including public certificate discovery and an MPass login.
5. Publish the draft as a prerelease. Wait for the Alpha channel workflow to succeed. Check that the manifest points to this numbered release and then test updating from the previous installed Alpha.

Never publish a draft left behind by a failed verification step. Retry the failed workflow or remove the draft first. Do not overwrite assets on an already published numbered release: ship a higher version instead.

[Documentation index](index.md)
