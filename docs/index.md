# Documentation

MoldSign Bridge connects MPass in your browser to a SafeNet token through a local macOS app. The app replaces the local server and approval interface needed for the tested MPass login flow.

| Start here                            | What you’ll find                                                   |
| ------------------------------------- | ------------------------------------------------------------------ |
| [Getting started](getting_started.md) | Install requirements, establish trust, and log in                  |
| [Architecture](architecture.md)       | Components, processes, lifecycle, and the login sequence           |
| [MPass protocol](mpass_protocol.md)   | HTTP contract, XAdES structure, and compatibility decisions        |
| [Security](security.md)               | Local trust, private-key boundaries, PIN handling, and limitations |
| [Development](development.md)         | Build commands, source map, imports, formatting, and tests         |
| [Troubleshooting](troubleshooting.md) | Diagnose connection failures and authentication errors             |

## Confirmed scope

The user confirmed a successful end-to-end MPass USB-token login on **November 3, 2026**, on an Apple Silicon Mac with a SafeNet ID Prime MD token. Local certificate discovery, PIN approval, signature creation, and MPass acceptance worked together in that environment.

This is an interoperability result for that configuration, not certification of all SafeNet devices or all MoldSign APIs. Document signing, decryption, other government origins, and unattended signing are outside the implemented scope.

## Keeping these pages accurate

The source code is authoritative for current behavior. Update the relevant page when changing endpoints, certificate trust, signature serialization, process limits, or frontend structure. Keep sample payloads synthetic; do not add real certificates, signatures, token identifiers, PINs, or browser session data.

[Back to the README](../README.md)
