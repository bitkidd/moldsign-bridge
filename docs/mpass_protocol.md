# MPass protocol

This page describes the local API implemented for the observed MPass desktop USB-token flow. It is not a specification of the complete MoldSign API.

## Connection and request guards

| Setting            | Value                            |
| ------------------ | -------------------------------- |
| Browser origin     | `https://mpass.gov.md`           |
| Local base URL     | `https://localhost.cts.md:18443` |
| Listener           | `127.0.0.1:18443`                |
| Body limit         | 16 KiB                           |
| Request lifetime   | 120 seconds from creation        |
| Pending operations | One                              |

Host validation accepts HTTP/1.1’s `Host` header and HTTP/2’s URI authority, while rejecting conflicting values. This distinction matters: Safari’s HTTP/2 requests originally reached the service but failed the HTTP/1-only guard.

CORS permits the exact MPass origin, `GET`, `POST`, and `OPTIONS`, with the allowed request headers `Content-Type` and `Accept`. It exposes `Location`, `sessionId`, and `error`. Preflight supports private-network access. Browser local-network permissions may still require user approval. Responses use `Cache-Control: no-store`.

Origin is mandatory for certificate, signing, and result requests. `/health` is the exception: it permits a missing origin but still requires the expected host. No HTTP or LAN listener is provided.

## 1. List certificates

```http
GET /certificates?private_only=true
Origin: https://mpass.gov.md
```

The response is a JSON object with a `certificateModel` array. Each model includes `certificateId`, `providerId`, `label`, `certificateName`, `certificateBase64`, `subjectDN`, `issuerDN`, `privateKeyPresent`, and `pinFinalTry`.

The ID is the certificate’s SHA-256 fingerprint. The provider ID is `PKCS11`. DER bytes and names originate from the local token. The display-oriented DN fields contain the names used by this alpha, not full general-purpose directory-name serialization.

Only time-valid certificates from tokens that do not report a locked PIN are listed. The bridge cannot prove that a corresponding private key is accessible before token login: key matching is confirmed during signing. This endpoint is a limited compatibility implementation, not a general certificate-validation or revocation service.

## 2. Request a login signature

```http
POST /sign/data
Origin: https://mpass.gov.md
Content-Type: application/json
```

Illustrative payload; the certificate ID must come from the preceding listing:

```json
{
  "algorithm": "SHA-1",
  "certificate": { "certificateId": "<ID returned by this app>" },
  "signatureType": "Embedded",
  "contentType": "Text",
  "signFormat": "XAdES-T",
  "data": "AAAAAAAAAAAAAAAAAAAAAAAAAAA="
}
```

The example is a synthetic 20-byte challenge, not a real login request. The bridge accepts only the combination above, with exactly 20 decoded challenge bytes. Unknown top-level fields and other signing formats are rejected. The cached local model supplies the actual certificate; browser-supplied certificate bytes are not used for signing.

The response is **201 Created**, an empty body, and these headers:

```http
Location: https://localhost.cts.md:18443/sign/data/PKCS11/<operation-id>/XAdES
sessionId: <operation-id>
Content-Type: application/json
```

The empty body is intentional. The observed MPass jQuery flow expects JSON, encounters the empty 201 response in its failure callback, checks status 201, then follows `Location`. Returning a normal JSON success body here changes that behavior.

## 3. Await approval and retrieve the result

```http
GET /sign/data/PKCS11/<operation-id>/XAdES
Origin: https://mpass.gov.md
```

The request waits for approval, cancellation, failure, or expiry. Success returns:

```json
{ "base64File": "<Base64 of the complete UTF-8 XAdES XML document>" }
```

The result is consumed once. Errors use HTTP status plus an exposed `error` header. For example, cancellation returns 400, expiry returns 408, and an unknown or consumed operation returns 404. A second request while an operation remains active receives 409.

The browser submits `base64File` to MPass’s own `/moldsign/login` endpoint. The app does not submit government credentials or manage the browser’s login session.

## Signature construction

`src-tauri/src/xades.rs` builds the XML envelope. `token_sign.rs` produces and verifies its RSA signature.

1. Decode the browser’s challenge once.
2. Embed it as Base64 text in `ds:Object` with an ID beginning `FileObject-`.
3. Add XAdES signing time and signing-certificate properties, including certificate digest, issuer name, and serial number.
4. Digest the signed properties and embedded object; reference both from `SignedInfo`.
5. Sign canonical `SignedInfo` using RSA PKCS#1 v1.5 with SHA-1, as requested by this MPass flow.
6. Obtain a SHA-256 timestamp over canonical `SignatureValue`.
7. Return the complete XML document encoded once as Base64.

The worker checks signing capability flags, rather than relying solely on the driver’s mechanism list. It prefers `CKM_RSA_PKCS` with a locally constructed SHA-1 `DigestInfo`. A token that supports only the combined mechanism can use `CKM_SHA1_RSA_PKCS`. Both produce the same signature algorithm. Fallback is limited to unsupported mechanism errors at `SignInit`, before a signature exists; it does not repeat token login or retry the PIN.

## Serialization details required by the tested flow

| Element                                | Compatibility profile                                           |
| -------------------------------------- | --------------------------------------------------------------- |
| `SignedInfo`                           | Canonical XML 1.0 with comments                                 |
| Reference digests                      | Implicit Canonical XML 1.0; no explicit transforms              |
| Certificate issuer                     | RFC 4514 direction and escaping, compatible with RFC 2253 names |
| `ds:Object` ordering                   | Qualifying properties before the embedded challenge             |
| `SignatureValue` timestamp input       | Canonical XML 1.0                                               |
| Signature, certificate, timestamp text | Base64 wrapped at 64 columns                                    |
| `EncapsulatedTimeStamp`                | Explicit `http://uri.etsi.org/01903/v1.2.2#DER` encoding        |

The installed server’s `Signature`, `SignXAdESWorker`, `SignatureInstance`, `SignatureFile`, and transfer-object classes were inspected to establish this profile. The app does not load those classes at runtime. Its former proprietary wrapper was removed.

## Timestamp service

The inspected endpoint is:

```text
http://tsp.pki.gov.md/moldsign2/<certificate-serial-in-reversed-byte-order>
```

The request is an RFC 3161-style timestamp query containing a SHA-256 imprint, nonce, and request for certificates. The URL identifies the certificate serial. The response is bounded and has a 15-second timeout. Non-success responses, rejected status, malformed structures, and mismatched nonce/imprint are rejected locally.

The production implementation does **not** independently validate the TSA CMS signature or certificate trust chain. MPass remains responsible for relying-party validation. The development test does independently verify a disposable timestamp signature, imprint, and test trust chain; that is a test of construction, not validation of the live TSA’s trust.

[Documentation index](index.md)
