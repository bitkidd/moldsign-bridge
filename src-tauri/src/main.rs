#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod bridge;
mod token_sign;
mod xades;
use base64::Engine;
use serde::Serialize;
use std::{
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use token_sign::run as run_sign;

const DRIVERS: &[&str] = &[
    "/Library/Frameworks/eToken.framework/Versions/A/libeToken.dylib",
    "/usr/local/lib/libeToken.dylib",
];
fn driver_path() -> Option<PathBuf> {
    DRIVERS.iter().map(PathBuf::from).find(|p| p.is_file())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Inspection {
    driver: bool,
    driver_path: Option<String>,
    architecture: &'static str,
}
#[tauri::command]
fn inspect() -> Inspection {
    let path = driver_path();
    Inspection {
        driver: path.is_some(),
        driver_path: path.map(|p| p.display().to_string()),
        architecture: std::env::consts::ARCH,
    }
}

fn token_probe() -> Result<serde_json::Value, String> {
    use cryptoki::{
        context::{CInitializeArgs, CInitializeFlags, Pkcs11},
        object::{Attribute, AttributeType, CertificateType, ObjectClass},
    };
    use sha2::{Digest, Sha256};
    use x509_parser::prelude::*;
    let path = driver_path()
        .ok_or("Install SafeNet Authentication Client for macOS to access the token.")?;
    let module = Pkcs11::new(path).map_err(|e| format!("Cannot load the SafeNet driver: {e}"))?;
    module
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .map_err(|e| e.to_string())?;
    let mut tokens = Vec::new();
    for slot in module
        .get_slots_with_token()
        .map_err(|e| e.to_string())?
        .into_iter()
        .take(16)
    {
        let info = module.get_token_info(slot).map_err(|e| e.to_string())?;
        let session = module.open_ro_session(slot).map_err(|e| e.to_string())?;
        let template = [
            Attribute::Class(ObjectClass::CERTIFICATE),
            Attribute::CertificateType(CertificateType::X_509),
        ];
        let objects = session.iter_objects(&template).map_err(|e| e.to_string())?;
        let mut certificates = Vec::new();
        for object in objects.take(256) {
            let object = object.map_err(|e| e.to_string())?;
            for attr in session
                .get_attributes(object, &[AttributeType::Value])
                .map_err(|e| e.to_string())?
            {
                if let Attribute::Value(der) = attr {
                    if der.len() > 1_048_576 {
                        return Err("The token returned an oversized certificate.".into());
                    }
                    let (_, cert) = X509Certificate::from_der(&der)
                        .map_err(|_| "The token returned an invalid X.509 certificate.")?;
                    let name = |dn: &X509Name<'_>| {
                        dn.iter_common_name()
                            .find_map(|a| a.as_str().ok())
                            .unwrap_or("Unnamed certificate")
                            .to_owned()
                    };
                    certificates.push(serde_json::json!({
                        "subject": name(cert.subject()), "issuer": name(cert.issuer()),
                        "expires": cert.validity().not_after.to_string(),
                        "expiresUnix": cert.validity().not_after.timestamp(),
                        "fingerprint": format!("{:x}", Sha256::digest(&der)),
                        "der": base64::engine::general_purpose::STANDARD.encode(&der),
                        "valid": cert.validity().is_valid()
                    }));
                }
            }
        }
        tokens.push(serde_json::json!({"label":info.label(),"manufacturer":info.manufacturer_id(),"model":info.model(),
            "pinLocked":info.user_pin_locked(),"pinFinalTry":info.user_pin_final_try(),"certificates":certificates}));
    }
    Ok(
        serde_json::json!({"status":if tokens.is_empty(){"no_token"}else{"connected"},"architecture":std::env::consts::ARCH,"tokens":tokens}),
    )
}

fn read_bounded(mut stream: impl Read) -> Vec<u8> {
    let mut result = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let keep = n.min(1_048_576usize.saturating_sub(result.len()));
                result.extend_from_slice(&buffer[..keep]);
            }
        }
    }
    result
}

fn run_probe() -> Result<serde_json::Value, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = Command::new(executable)
        .arg("--probe-token")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Cannot launch the helper: {e}"))?;
    let stdout = child.stdout.take().ok_or("Cannot capture helper output.")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("Cannot capture helper diagnostics.")?;
    let out = std::thread::spawn(move || read_bounded(stdout));
    let err = std::thread::spawn(move || read_bounded(stderr));
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            _ => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }
    let output = out.join().map_err(|_| "Cannot read helper output.")?;
    let diagnostics = err.join().unwrap_or_default();
    if timed_out {
        return Err("The token helper did not respond within 20 seconds. It has been stopped; no PIN was requested.".into());
    }
    serde_json::from_slice(&output).map_err(|_| {
        let detail = String::from_utf8_lossy(&diagnostics);
        format!(
            "The helper exited before returning a result. {}",
            detail.chars().take(1000).collect::<String>()
        )
    })
}

#[tauri::command]
async fn probe() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(run_probe)
        .await
        .map_err(|e| e.to_string())?
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--token-capabilities") {
        let result =
            token_sign::capabilities().unwrap_or_else(|error| serde_json::json!({"error": error}));
        println!("{}", result);
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--sign-token") {
        let result =
            token_sign::worker().unwrap_or_else(|error| serde_json::json!({"error":error}));
        println!("{}", result);
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--probe-token") {
        let result = token_probe().unwrap_or_else(
            |message| serde_json::json!({"status":"error","errorType":"SafeNet","message":message}),
        );
        println!("{}", result);
        return;
    }
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    tauri::Builder::default()
        .setup(bridge::initialize)
        .invoke_handler(tauri::generate_handler![
            inspect,
            probe,
            bridge::service_status,
            bridge::start_service,
            bridge::trust_connection,
            bridge::approve_login,
            bridge::reject_login,
            bridge::open_mpass
        ])
        .run(tauri::generate_context!())
        .expect("Unable to launch MoldSign Bridge Preview");
}
