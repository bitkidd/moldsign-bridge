use base64::{engine::general_purpose::STANDARD as B64, Engine};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use x509_parser::prelude::*;

const DS: &str = "http://www.w3.org/2000/09/xmldsig#";
const XADES: &str = "http://uri.etsi.org/01903/v1.3.2#";
const C14N: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
const SIGNED_INFO_C14N: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments";

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\r', "&#xD;")
}
fn canonical(xml: &str) -> Result<String, String> {
    xml_c14n::canonicalize_xml(
        xml,
        xml_c14n::CanonicalizationOptions {
            mode: xml_c14n::CanonicalizationMode::Canonical1_0,
            keep_comments: false,
            inclusive_ns_prefixes: vec![],
        },
    )
    .map_err(|_| "Cannot canonicalize login signature.".into())
}
fn digest(xml: &str) -> Result<String, String> {
    Ok(B64.encode(Sha1::digest(canonical(xml)?.as_bytes())))
}

pub struct Prepared {
    pub signed_info: String,
    properties: String,
    object: String,
    certificate: String,
    id: String,
    serial: String,
}
impl Prepared {
    pub fn new(der: &[u8], challenge: &[u8], id: &str) -> Result<Self, String> {
        if challenge.len() != 20 {
            return Err("Unexpected MPass challenge length.".into());
        }
        let (_, cert) = X509Certificate::from_der(der).map_err(|_| "Invalid certificate.")?;
        if !cert.validity().is_valid() {
            return Err("The selected certificate has expired or is not yet valid.".into());
        }
        let serial = num_bigint::BigUint::from_bytes_be(cert.raw_serial()).to_str_radix(10);
        let issuer = escape(&issuer_name(cert.issuer().as_raw())?);
        let now = ::time::OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .map_err(|e| e.to_string())?
            .format(&::time::format_description::well_known::Rfc3339)
            .map_err(|e| e.to_string())?;
        let cert_digest = B64.encode(Sha1::digest(der));
        let properties = format!(
            r##"<xades:SignedProperties xmlns:ds="{DS}" xmlns:xades="{XADES}" Id="SignedProperties-{id}"><xades:SignedSignatureProperties><xades:SigningTime>{now}</xades:SigningTime><xades:SigningCertificate><xades:Cert><xades:CertDigest><ds:DigestMethod Algorithm="{DS}sha1"></ds:DigestMethod><ds:DigestValue>{cert_digest}</ds:DigestValue></xades:CertDigest><xades:IssuerSerial><ds:X509IssuerName>{issuer}</ds:X509IssuerName><ds:X509SerialNumber>{serial}</ds:X509SerialNumber></xades:IssuerSerial></xades:Cert></xades:SigningCertificate></xades:SignedSignatureProperties><xades:SignedDataObjectProperties Id="SignedDataObjectProperties-{id}"><xades:DataObjectFormat ObjectReference="SignedFile-Reference-{id}-login-challenge.tmp"><xades:Description>login-challenge.tmp</xades:Description><xades:MimeType>application/octet-stream</xades:MimeType></xades:DataObjectFormat></xades:SignedDataObjectProperties></xades:SignedProperties>"##
        );
        let data = B64.encode(challenge);
        let object = format!(
            r#"<ds:Object xmlns:ds="{DS}" Encoding="UTF-8" Id="FileObject-{id}">{data}</ds:Object>"#
        );
        let pd = digest(&properties)?;
        let od = digest(&object)?;
        let signed_info = canonical(&format!(
            r##"<ds:SignedInfo xmlns:ds="{DS}" Id="SignedInfo-{id}"><ds:CanonicalizationMethod Algorithm="{SIGNED_INFO_C14N}"></ds:CanonicalizationMethod><ds:SignatureMethod Algorithm="{DS}rsa-sha1"></ds:SignatureMethod><ds:Reference Id="SignedProperties-Reference-{id}" Type="http://uri.etsi.org/01903#SignedProperties" URI="#SignedProperties-{id}"><ds:DigestMethod Algorithm="{DS}sha1"></ds:DigestMethod><ds:DigestValue>{pd}</ds:DigestValue></ds:Reference><ds:Reference Id="SignedFile-Reference-{id}-login-challenge.tmp" URI="#FileObject-{id}"><ds:DigestMethod Algorithm="{DS}sha1"></ds:DigestMethod><ds:DigestValue>{od}</ds:DigestValue></ds:Reference></ds:SignedInfo>"##
        ))?;
        // Moldova's timestamp endpoint uses the certificate serial in little-endian hex.
        let mut serial_bytes = cert.raw_serial().to_vec();
        serial_bytes.reverse();
        let serial = serial_bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Self {
            signed_info,
            properties,
            object,
            certificate: wrap_base64(&B64.encode(der)),
            id: id.to_owned(),
            serial,
        })
    }
    pub fn finish(self, signature: &[u8]) -> Result<String, String> {
        let id = &self.id;
        let signature_value = format!(
            r#"<ds:SignatureValue xmlns:ds="{DS}" Id="SignatureValue-{id}">{}</ds:SignatureValue>"#,
            wrap_base64(&B64.encode(signature))
        );
        let imprint = Sha256::digest(canonical(&signature_value)?.as_bytes());
        let timestamp = timestamp(&self.serial, &imprint)?;
        self.assemble(&signature_value, &timestamp)
    }
    fn assemble(&self, signature_value: &str, timestamp: &str) -> Result<String, String> {
        let id = &self.id;
        let timestamp = wrap_base64(timestamp);
        Ok(format!(
            r##"<?xml version="1.0" encoding="UTF-8"?><ds:Signature xmlns:ds="{DS}" Id="Signature-{id}">{}{signature_value}<ds:KeyInfo Id="KeyInfo-{id}"><ds:X509Data><ds:X509Certificate>{}</ds:X509Certificate></ds:X509Data></ds:KeyInfo><ds:Object><xades:QualifyingProperties xmlns:xades="{XADES}" Target="#Signature-{id}">{}<xades:UnsignedProperties><xades:UnsignedSignatureProperties><xades:SignatureTimeStamp Id="SignatureTimeStamp-{id}"><ds:CanonicalizationMethod Algorithm="{C14N}"></ds:CanonicalizationMethod><xades:EncapsulatedTimeStamp Encoding="http://uri.etsi.org/01903/v1.2.2#DER">{timestamp}</xades:EncapsulatedTimeStamp></xades:SignatureTimeStamp></xades:UnsignedSignatureProperties></xades:UnsignedProperties></xades:QualifyingProperties></ds:Object>{}</ds:Signature>"##,
            self.signed_info, self.certificate, self.properties, self.object
        ))
    }
}

fn wrap_base64(encoded: &str) -> String {
    encoded
        .as_bytes()
        .chunks(64)
        .map(|line| std::str::from_utf8(line).expect("base64 is ASCII"))
        .collect::<Vec<_>>()
        .join("\n")
}

// XMLDSig uses the LDAP/RFC 2253 (RFC 4514) direction and escaping for issuer names.
// X509Name's display formatter is intended for humans and writes RDNs in the opposite order.
fn issuer_name(encoded: &[u8]) -> Result<String, String> {
    use x509_cert::der::Decode;
    x509_cert::name::Name::from_der(encoded)
        .map(|name| name.to_string())
        .map_err(|_| "Cannot encode the certificate issuer for the login signature.".into())
}

fn der(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    if body.len() < 128 {
        out.push(body.len() as u8)
    } else {
        let bytes = body.len().to_be_bytes();
        let bytes = &bytes[bytes
            .iter()
            .position(|b| *b != 0)
            .unwrap_or(bytes.len() - 1)..];
        out.push(0x80 | bytes.len() as u8);
        out.extend_from_slice(bytes);
    }
    out.extend_from_slice(body);
    out
}
fn tlv(data: &[u8]) -> Result<(u8, &[u8], &[u8]), String> {
    if data.len() < 2 {
        return Err("Truncated timestamp response.".into());
    }
    let n = (data[1] & 0x7f) as usize;
    let (offset, len) = if data[1] & 0x80 == 0 {
        (2, data[1] as usize)
    } else {
        if n == 0 || n > 4 || data.len() < 2 + n {
            return Err("Invalid timestamp length.".into());
        }
        (
            2 + n,
            data[2..2 + n]
                .iter()
                .fold(0usize, |v, b| (v << 8) | *b as usize),
        )
    };
    if len > data.len().saturating_sub(offset) {
        return Err("Truncated timestamp response.".into());
    }
    Ok((data[0], &data[offset..offset + len], &data[offset + len..]))
}
fn fields(mut data: &[u8]) -> Result<Vec<(u8, &[u8])>, String> {
    let mut out = vec![];
    while !data.is_empty() {
        let (t, b, rest) = tlv(data)?;
        out.push((t, b));
        data = rest;
        if out.len() > 64 {
            return Err("Invalid timestamp structure.".into());
        }
    }
    Ok(out)
}
fn timestamp(serial: &str, imprint: &[u8]) -> Result<String, String> {
    let nonce = uuid::Uuid::new_v4();
    let mut nonce_bytes = vec![0];
    nonce_bytes.extend_from_slice(nonce.as_bytes());
    let alg = der(
        0x30,
        &[0x06, 9, 0x60, 0x86, 0x48, 1, 0x65, 3, 4, 2, 1, 0x05, 0],
    );
    let mi = der(0x30, &[alg, der(4, imprint)].concat());
    let request = der(
        0x30,
        &[vec![2, 1, 1], mi, der(2, &nonce_bytes), vec![1, 1, 0xff]].concat(),
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .post(format!("http://tsp.pki.gov.md/moldsign2/{serial}"))
        .header("Content-Type", "application/timestamp-query")
        .body(request)
        .send()
        .map_err(|_| "Moldova's timestamp service could not be reached. Try again.")?;
    if !response.status().is_success() {
        return Err("The timestamp service rejected the request.".into());
    }
    use std::io::Read;
    let mut bytes = vec![];
    response
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("Oversized timestamp response.".into());
    }
    extract_timestamp(&bytes, imprint, &nonce_bytes).map(|token| B64.encode(token))
}
fn extract_timestamp<'a>(
    response: &'a [u8],
    imprint: &[u8],
    nonce: &[u8],
) -> Result<&'a [u8], String> {
    let failure =
        || "The timestamp service returned an invalid or mismatched response.".to_string();
    let (tag, body, rest) = tlv(response)?;
    if tag != 0x30 || !rest.is_empty() {
        return Err(failure());
    }
    let (status_tag, status, token) = tlv(body)?;
    if status_tag != 0x30 {
        return Err(failure());
    }
    let (code_tag, code, _) = tlv(status)?;
    if code_tag != 2 || !(code == [0] || code == [1]) {
        return Err("The timestamp service declined this certificate.".into());
    }
    let (t, ci, remaining) = tlv(token)?;
    if t != 0x30 || !remaining.is_empty() {
        return Err(failure());
    }
    let ci = fields(ci)?;
    let content = ci.get(1).filter(|v| v.0 == 0xa0).ok_or_else(failure)?;
    let (t, sd, _) = tlv(content.1)?;
    if t != 0x30 {
        return Err(failure());
    }
    let sd = fields(sd)?;
    let enc = sd.get(2).filter(|v| v.0 == 0x30).ok_or_else(failure)?;
    let enc = fields(enc.1)?;
    let ec = enc.get(1).filter(|v| v.0 == 0xa0).ok_or_else(failure)?;
    let (t, octets, _) = tlv(ec.1)?;
    if t != 4 {
        return Err(failure());
    }
    let (t, info, _) = tlv(octets)?;
    if t != 0x30 {
        return Err(failure());
    }
    let info = fields(info)?;
    let mi = info.get(2).filter(|v| v.0 == 0x30).ok_or_else(failure)?;
    let mi = fields(mi.1)?;
    if mi.get(1).map(|v| (v.0, v.1)) != Some((4, imprint)) {
        return Err(failure());
    }
    let expected = nonce
        .iter()
        .skip_while(|b| **b == 0)
        .copied()
        .collect::<Vec<_>>();
    let actual = info
        .iter()
        .skip(5)
        .find(|v| v.0 == 2)
        .ok_or_else(failure)?
        .1
        .iter()
        .skip_while(|b| **b == 0)
        .copied()
        .collect::<Vec<_>>();
    if expected != actual {
        return Err(failure());
    }
    // The relying party validates TSA chain/signature; locally we bind the response to its nonce and imprint.
    Ok(token)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_timestamp_is_rejected() {
        for bytes in [
            &[][..],
            &[0x30, 0x80][..],
            &[0x30, 0x84, 0xff, 0xff, 0xff, 0xff][..],
            &[0x30, 2, 2, 1][..],
        ] {
            assert!(extract_timestamp(bytes, &[0; 32], &[1]).is_err());
        }
    }
    #[test]
    fn xml_injection_is_escaped() {
        assert_eq!(escape("<&\"\r"), "&lt;&amp;&quot;&#xD;");
    }
}

#[cfg(test)]
mod interoperability {
    use super::*;
    #[test]
    fn export_software_key_signature_for_independent_verifier() {
        let Some(directory) = std::env::var_os("BRIDGE_INTEROP_DIR") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        let der = std::fs::read(directory.join("cert.der")).unwrap();
        let prepared = Prepared::new(&der, &[42; 20], "interop-test").unwrap();
        std::fs::write(directory.join("signed-info.xml"), &prepared.signed_info).unwrap();
        let status = std::process::Command::new("/usr/bin/openssl")
            .args(["dgst", "-sha1", "-sign"])
            .arg(directory.join("key.pem"))
            .arg("-out")
            .arg(directory.join("signature.bin"))
            .arg(directory.join("signed-info.xml"))
            .status()
            .unwrap();
        assert!(status.success());
        let signature = std::fs::read(directory.join("signature.bin")).unwrap();
        let value = format!(
            r#"<ds:SignatureValue xmlns:ds="{DS}" Id="SignatureValue-interop-test">{}</ds:SignatureValue>"#,
            wrap_base64(&B64.encode(signature))
        );
        let timestamp = if let Some(config) = std::env::var_os("BRIDGE_TEST_TSA_CONFIG") {
            let timestamp_input = canonical(&value).unwrap();
            std::fs::write(directory.join("timestamp-input.xml"), &timestamp_input).unwrap();
            let run = |arguments: Vec<std::ffi::OsString>| {
                assert!(std::process::Command::new("/usr/bin/openssl")
                    .args(arguments)
                    .status()
                    .unwrap()
                    .success());
            };
            run(vec![
                "ts".into(),
                "-query".into(),
                "-data".into(),
                directory.join("timestamp-input.xml").into(),
                "-sha256".into(),
                "-cert".into(),
                "-out".into(),
                directory.join("request.tsq").into(),
            ]);
            run(vec![
                "ts".into(),
                "-reply".into(),
                "-config".into(),
                config,
                "-queryfile".into(),
                directory.join("request.tsq").into(),
                "-out".into(),
                directory.join("response.tsr").into(),
            ]);
            run(vec![
                "ts".into(),
                "-reply".into(),
                "-in".into(),
                directory.join("response.tsr").into(),
                "-token_out".into(),
                "-out".into(),
                directory.join("timestamp.der").into(),
            ]);
            let response = std::fs::read(directory.join("response.tsr")).unwrap();
            let request = std::fs::read(directory.join("request.tsq")).unwrap();
            let (_, body, _) = tlv(&request).unwrap();
            let request_fields = fields(body).unwrap();
            let nonce = request_fields
                .iter()
                .skip(2)
                .find(|field| field.0 == 2)
                .unwrap()
                .1;
            let token = extract_timestamp(
                &response,
                &Sha256::digest(timestamp_input.as_bytes()),
                nonce,
            )
            .unwrap();
            assert_eq!(
                token,
                std::fs::read(directory.join("timestamp.der")).unwrap()
            );
            B64.encode(token)
        } else {
            String::new()
        };
        let xml = prepared.assemble(&value, &timestamp).unwrap();
        std::fs::write(directory.join("signature.xml"), xml).unwrap();
    }
}

#[cfg(test)]
mod issuer_tests {
    use super::*;
    use std::str::FromStr;
    use x509_cert::der::Encode;
    #[test]
    fn issuer_name_preserves_ldap_order_and_escaping() {
        let expected = r"CN=Issuer\, Inc.,O=Test Authority,C=MD";
        let name = x509_cert::name::Name::from_str(expected).unwrap();
        let encoded = name.to_der().unwrap();
        assert_eq!(issuer_name(&encoded).unwrap(), expected);
        let (_, parsed) = X509Name::from_der(&encoded).unwrap();
        assert_ne!(parsed.to_string(), expected);
    }
}
