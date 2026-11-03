use base64::{engine::general_purpose::STANDARD as B64, Engine};
use cryptoki::{
    context::{CInitializeArgs, CInitializeFlags, Pkcs11},
    mechanism::{Mechanism, MechanismType},
    object::{Attribute, AttributeType, ObjectClass},
    session::UserType,
    types::AuthPin,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    fingerprint: String,
    data: String,
    pin: String,
    final_try_confirmed: bool,
}
pub fn worker() -> Result<serde_json::Value, String> {
    let mut input = Zeroizing::new(String::new());
    std::io::stdin()
        .take(65_537)
        .read_to_string(&mut input)
        .map_err(|_| "Cannot read signing request.")?;
    if input.len() > 65_536 {
        return Err("Oversized signing request.".into());
    }
    let request: Input = serde_json::from_str(&input).map_err(|_| "Invalid signing request.")?;
    drop(input);
    let pin = Zeroizing::new(request.pin);
    let data = B64
        .decode(request.data)
        .map_err(|_| "Invalid signing data.")?;
    if data.len() > 32_768 || pin.is_empty() || pin.len() > 128 {
        return Err("Invalid signing request.".into());
    }
    let path = crate::driver_path().ok_or("SafeNet driver is unavailable.")?;
    let module = Pkcs11::new(path).map_err(|_| "Cannot load SafeNet driver.")?;
    module
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .map_err(|e| e.to_string())?;
    for slot in module
        .get_slots_with_token()
        .map_err(|e| e.to_string())?
        .into_iter()
        .take(16)
    {
        let session = module.open_ro_session(slot).map_err(|e| e.to_string())?;
        let objects = session
            .find_objects(&[Attribute::Class(ObjectClass::CERTIFICATE)])
            .map_err(|e| e.to_string())?;
        for object in objects.into_iter().take(256) {
            let attributes = session
                .get_attributes(object, &[AttributeType::Value, AttributeType::Id])
                .map_err(|e| e.to_string())?;
            let mut der = None;
            let mut id = None;
            for attribute in attributes {
                match attribute {
                    Attribute::Value(v) => der = Some(v),
                    Attribute::Id(v) => id = Some(v),
                    _ => {}
                }
            }
            let Some(der) = der else { continue };
            if format!("{:x}", Sha256::digest(&der)) != request.fingerprint {
                continue;
            }
            let id = id
                .filter(|id| !id.is_empty())
                .ok_or("The certificate has no associated key identifier.")?;
            let info = module.get_token_info(slot).map_err(|e| e.to_string())?;
            if info.user_pin_locked() {
                return Err("Your token PIN is locked. Contact your token provider.".into());
            }
            if info.user_pin_final_try() && !request.final_try_confirmed {
                return Err("SafeNet now reports one PIN attempt remains. Refresh MPass and confirm the warning before trying again.".into());
            }
            use x509_parser::prelude::FromDer;
            let (_, cert) = x509_parser::certificate::X509Certificate::from_der(&der)
                .map_err(|_| "Invalid token certificate.")?;
            if cert.public_key().algorithm.algorithm.to_id_string() != "1.2.840.113549.1.1.1" {
                return Err("This MPass flow requires an RSA token certificate.".into());
            }
            let mechanisms = module.get_mechanism_list(slot).map_err(|e| e.to_string())?;
            let can_sign = |mechanism| {
                mechanisms.contains(&mechanism)
                    && module
                        .get_mechanism_info(slot, mechanism)
                        .map(|info| info.sign())
                        .unwrap_or(false)
            };
            let raw_rsa = can_sign(MechanismType::RSA_PKCS);
            let combined_sha1 = can_sign(MechanismType::SHA1_RSA_PKCS);
            if !raw_rsa && !combined_sha1 {
                return Err(
                    "Your token does not provide an RSA signing method compatible with MPass."
                        .into(),
                );
            }
            // Exactly one PIN attempt. This process exits after signing and never caches a login session.
            session
                .login(UserType::User, Some(&AuthPin::new(pin.to_string().into())))
                .map_err(|e| match e {
                    cryptoki::error::Error::Pkcs11(cryptoki::error::RvError::PinIncorrect, _) => {
                        "Incorrect PIN. No retry was made; check SafeNet before trying again."
                            .into()
                    }
                    cryptoki::error::Error::Pkcs11(cryptoki::error::RvError::PinLocked, _) => {
                        "The token PIN is locked.".into()
                    }
                    _ => format!("SafeNet login failed: {e}"),
                })?;
            let signed = (|| {
                let keys = session
                    .find_objects(&[
                        Attribute::Class(ObjectClass::PRIVATE_KEY),
                        Attribute::Id(id),
                    ])
                    .map_err(|e| e.to_string())?;
                if keys.len() != 1 {
                    return Err("Cannot uniquely identify the certificate's private key.".into());
                }
                sign_compatible(raw_rsa, combined_sha1, &data, |mechanism, payload| {
                    session.sign(mechanism, keys[0], payload)
                })
                .map_err(|error| signing_error(&error))
            })();
            let _ = session.logout();
            let signature = signed?;

            let (_, cert) = x509_parser::certificate::X509Certificate::from_der(&der)
                .map_err(|_| "Invalid token certificate.")?;
            ring::signature::UnparsedPublicKey::new(
                &ring::signature::RSA_PKCS1_1024_8192_SHA1_FOR_LEGACY_USE_ONLY,
                cert.public_key().subject_public_key.data.as_ref(),
            )
            .verify(&data, &signature)
            .map_err(|_| "The token signature could not be verified against the certificate.")?;
            return Ok(serde_json::json!({"signature":B64.encode(signature)}));
        }
    }
    Err("The selected token or certificate is no longer connected.".into())
}
pub fn run(
    fingerprint: &str,
    data: &[u8],
    pin: &str,
    final_try_confirmed: bool,
) -> Result<Vec<u8>, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = Command::new(executable)
        .arg("--sign-token")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Cannot launch the isolated signer.")?;
    #[derive(serde::Serialize)]
    struct BorrowedInput<'a> {
        fingerprint: &'a str,
        data: String,
        pin: &'a str,
        final_try_confirmed: bool,
    }
    let request = Zeroizing::new(
        serde_json::to_vec(&BorrowedInput {
            fingerprint,
            data: B64.encode(data),
            pin,
            final_try_confirmed,
        })
        .map_err(|_| "Cannot prepare signing request.")?,
    );
    let write_result = child
        .stdin
        .take()
        .ok_or("Signer input unavailable.")?
        .write_all(&request);
    if write_result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Cannot send the PIN to the isolated signer.".into());
    }
    let stdout = child.stdout.take().ok_or("Signer output unavailable.")?;
    let output = std::thread::spawn(move || crate::read_bounded(stdout));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(25);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(100))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = output.join();
                return Err(
                    "SafeNet did not respond. The signer was stopped; your PIN was not retried."
                        .into(),
                );
            }
        }
    }
    let result: serde_json::Value =
        serde_json::from_slice(&output.join().map_err(|_| "Cannot read signer result.")?)
            .map_err(|_| "The signer exited without a result.")?;
    if let Some(error) = result["error"].as_str() {
        return Err(error.to_owned());
    }
    let signature = B64
        .decode(result["signature"].as_str().ok_or("Missing signature.")?)
        .map_err(|_| "Invalid signature.")?;

    Ok(signature)
}

// Both paths produce the same RSA PKCS#1 v1.5 / SHA-1 signature required by MPass.
// Some SafeNet tokens advertise combined hashing mechanisms but reject them at SignInit.
fn sha1_digest_info(data: &[u8]) -> Vec<u8> {
    let mut encoded = vec![
        0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14,
    ];
    encoded.extend_from_slice(&sha1::Sha1::digest(data));
    encoded
}
fn sign_compatible(
    raw_rsa: bool,
    combined_sha1: bool,
    data: &[u8],
    mut sign: impl FnMut(&Mechanism<'_>, &[u8]) -> cryptoki::error::Result<Vec<u8>>,
) -> cryptoki::error::Result<Vec<u8>> {
    use cryptoki::{
        context::Function,
        error::{Error, RvError},
    };
    if raw_rsa {
        match sign(&Mechanism::RsaPkcs, &sha1_digest_info(data)) {
            // Only initialization rejection is safe to fall back from: no signature exists yet.
            Err(Error::Pkcs11(
                RvError::MechanismInvalid | RvError::MechanismParamInvalid,
                Function::SignInit,
            )) if combined_sha1 => {}
            result => return result,
        }
    }
    if combined_sha1 {
        sign(&Mechanism::Sha1RsaPkcs, data)
    } else {
        Err(Error::Pkcs11(RvError::MechanismInvalid, Function::SignInit))
    }
}
fn signing_error(error: &cryptoki::error::Error) -> String {
    use cryptoki::error::{Error, RvError};
    match error {
        Error::Pkcs11(RvError::MechanismInvalid | RvError::MechanismParamInvalid, _) =>
            "The token rejected the available MPass signing methods. No signature was created. Your PIN was not retried.".into(),
        _ => format!("SafeNet could not complete the signature ({error}). Your PIN was not retried."),
    }
}

pub fn capabilities() -> Result<serde_json::Value, String> {
    let module = Pkcs11::new(crate::driver_path().ok_or("SafeNet driver unavailable.")?)
        .map_err(|e| e.to_string())?;
    module
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .map_err(|e| e.to_string())?;
    let mut tokens = Vec::new();
    for slot in module.get_slots_with_token().map_err(|e| e.to_string())? {
        let mechanisms = module.get_mechanism_list(slot).map_err(|e| e.to_string())?;
        let supported = |mechanism| {
            mechanisms.contains(&mechanism)
                && module
                    .get_mechanism_info(slot, mechanism)
                    .map(|info| info.sign())
                    .unwrap_or(false)
        };
        tokens.push(serde_json::json!({"rsaPkcsSign":supported(MechanismType::RSA_PKCS),"sha1RsaPkcsSign":supported(MechanismType::SHA1_RSA_PKCS)}));
    }
    Ok(serde_json::json!({"tokens":tokens,"pinRequested":false}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cryptoki::{
        context::Function,
        error::{Error, RvError},
    };
    #[test]
    fn raw_rsa_receives_standard_digest_info_not_plain_data() {
        let result = sign_compatible(true, true, b"abc", |mechanism, payload| {
            assert!(matches!(mechanism, Mechanism::RsaPkcs));
            let actual = payload
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            assert_eq!(
                actual,
                "3021300906052b0e03021a05000414a9993e364706816aba3e25717850c26c9cd0d89d"
            );
            Ok(vec![1])
        })
        .unwrap();
        assert_eq!(result, vec![1]);
    }
    #[test]
    fn unsupported_initialization_can_use_combined_method_in_same_session() {
        let mut calls = 0;
        let result = sign_compatible(true, true, b"abc", |mechanism, payload| {
            calls += 1;
            if calls == 1 {
                return Err(Error::Pkcs11(RvError::MechanismInvalid, Function::SignInit));
            }
            assert!(matches!(mechanism, Mechanism::Sha1RsaPkcs));
            assert_eq!(payload, b"abc");
            Ok(vec![2])
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(result, vec![2]);
    }
    #[test]
    fn does_not_retry_after_signing_or_authentication_errors() {
        for (code, function) in [
            (RvError::MechanismInvalid, Function::Sign),
            (RvError::PinIncorrect, Function::SignInit),
            (RvError::DeviceError, Function::SignInit),
        ] {
            let mut calls = 0;
            assert!(sign_compatible(true, true, b"abc", |_, _| {
                calls += 1;
                Err(Error::Pkcs11(code, function))
            })
            .is_err());
            assert_eq!(calls, 1);
        }
    }
    #[test]
    fn combined_only_token_receives_original_message() {
        sign_compatible(false, true, b"abc", |mechanism, data| {
            assert!(matches!(mechanism, Mechanism::Sha1RsaPkcs));
            assert_eq!(data, b"abc");
            Ok(vec![3])
        })
        .unwrap();
    }
}
