use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::Manager;
const ORIGIN: &str = "https://mpass.gov.md";
const BASE: &str = "https://localhost.cts.md:18443";
const REQUEST_TTL: Duration = Duration::from_secs(120);
const TRUST_CACHE_TTL: Duration = Duration::from_secs(30);

pub struct Bridge {
    app: tauri::AppHandle,
    directory: PathBuf,
    started: AtomicBool,
    scan_busy: AtomicBool,
    trust_cache: Mutex<Option<(Instant, bool)>>,
    inner: Mutex<Inner>,
}
struct Inner {
    updating: bool,
    certificates: HashMap<String, Value>,
    pending: Option<Operation>,
    last_message: String,
}
fn reserve_update(inner: &mut Inner) -> Result<(), String> {
    if inner.updating || inner.pending.is_some() {
        return Err("Finish or cancel the current login before updating.".into());
    }
    inner.updating = true;
    Ok(())
}
struct Operation {
    id: String,
    created: Instant,
    certificate: Value,
    challenge: Vec<u8>,
    signing: bool,
    result: Option<Result<String, String>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    running: bool,
    trusted: bool,
    certificate_path: String,
    message: String,
    pending: Option<Value>,
}
impl Bridge {
    pub fn new(directory: PathBuf, app: tauri::AppHandle) -> Self {
        Self {
            app,
            directory,
            started: AtomicBool::new(false),
            scan_busy: AtomicBool::new(false),
            trust_cache: Mutex::new(None),
            inner: Mutex::new(Inner {
                updating: false,
                certificates: HashMap::new(),
                pending: None,
                last_message: "Start the login connection when you are ready.".into(),
            }),
        }
    }
    pub fn begin_update(&self) -> Result<(), String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "Login service unavailable.")?;
        reserve_update(&mut inner)
    }
    pub fn end_update(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.updating = false;
        }
    }
    fn release_attention(&self) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.set_always_on_top(false);
        }
    }
    fn certificate(&self) -> PathBuf {
        self.directory.join("localhost.pem")
    }
    fn key(&self) -> PathBuf {
        self.directory.join("localhost-key.pem")
    }
    fn ensure_certificate(&self) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        std::fs::set_permissions(&self.directory, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        if self.certificate().is_file() && self.key().is_file() {
            return Ok(());
        }
        let cert = rcgen::generate_simple_self_signed(vec!["localhost.cts.md".into()])
            .map_err(|e| e.to_string())?;
        // A unique key per installation; never use MoldSign's shared localhost certificate.
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.key())
            .map_err(|e| e.to_string())?;
        std::fs::set_permissions(self.key(), std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
        file.write_all(cert.signing_key.serialize_pem().as_bytes())
            .map_err(|e| e.to_string())?;
        std::fs::write(self.certificate(), cert.cert.pem()).map_err(|e| e.to_string())?;
        Ok(())
    }
    fn trusted(&self) -> bool {
        let mut cache = match self.trust_cache.lock() {
            Ok(cache) => cache,
            Err(_) => return false,
        };
        if let Some((checked, trusted)) = *cache {
            if checked.elapsed() < TRUST_CACHE_TTL {
                return trusted;
            }
        }
        let trusted = self.verify_trust();
        *cache = Some((Instant::now(), trusted));
        trusted
    }
    fn verify_trust(&self) -> bool {
        self.certificate().is_file()
            && std::process::Command::new("/usr/bin/security")
                .args(["verify-cert", "-c"])
                .arg(self.certificate())
                .args(["-p", "ssl", "-s", "localhost.cts.md"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
    }
}
#[tauri::command]
pub async fn service_status(
    bridge: tauri::State<'_, Arc<Bridge>>,
) -> Result<ServiceStatus, String> {
    let bridge = bridge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let trusted = bridge.trusted();
        let mut inner = bridge
            .inner
            .lock()
            .map_err(|_| "Service state unavailable.")?;
        if inner
            .pending
            .as_ref()
            .is_some_and(|p| p.created.elapsed() > REQUEST_TTL)
        {
            inner.pending = None;
            inner.last_message = "The login request expired. Start again on MPass.".into();
            bridge.release_attention();
        }
        let pending = inner
            .pending
            .as_ref()
            .filter(|p| p.result.is_none())
            .map(|p| {
                json!({
                    "id": p.id, "origin": ORIGIN, "name": p.certificate["certificateName"],
                    "signing": p.signing, "pinFinalTry": p.certificate["pinFinalTry"]
                })
            });
        Ok(ServiceStatus {
            running: bridge.started.load(Ordering::SeqCst),
            trusted,
            certificate_path: bridge.certificate().display().to_string(),
            message: inner.last_message.clone(),
            pending,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn start_service(bridge: tauri::State<'_, Arc<Bridge>>) -> Result<(), String> {
    let bridge = bridge.inner().clone();
    if bridge.started.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    if let Err(e) = bridge.ensure_certificate() {
        bridge.started.store(false, Ordering::SeqCst);
        return Err(e);
    }
    let config = match axum_server::tls_rustls::RustlsConfig::from_pem_file(
        bridge.certificate(),
        bridge.key(),
    )
    .await
    {
        Ok(c) => c,
        Err(_) => {
            bridge.started.store(false, Ordering::SeqCst);
            return Err("Cannot load the local HTTPS certificate.".into());
        }
    };
    let listener = match std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 18443)) {
        Ok(l) => l,
        Err(_) => {
            bridge.started.store(false, Ordering::SeqCst);
            return Err("Port 18443 is already in use. Quit MoldSign Server or another copy of this app, then try again.".into());
        }
    };
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let router = Router::new()
        .route(
            "/health",
            get(|| async { Json(json!({"service":"MoldSign Bridge","mode":"MPass login only"})) }),
        )
        .route("/certificates", get(certificates))
        .route("/sign/data", post(sign))
        .route("/sign/data/PKCS11/{id}/XAdES", get(result))
        .layer(DefaultBodyLimit::max(16_384))
        .layer(middleware::from_fn(guard))
        .with_state(bridge.clone());
    bridge
        .inner
        .lock()
        .map_err(|_| "Service state unavailable.")?
        .last_message = "Waiting for MPass. Select USB token on the website.".into();
    let server = axum_server::from_tcp_rustls(listener, config).map_err(|e| {
        bridge.started.store(false, Ordering::SeqCst);
        e.to_string()
    })?;
    tauri::async_runtime::spawn(async move {
        if let Err(_) = server.serve(router.into_make_service()).await {
            if let Ok(mut inner) = bridge.inner.lock() {
                inner.last_message = "The local HTTPS connection stopped. Restart the app.".into();
            }
        }
        bridge.started.store(false, Ordering::SeqCst);
    });
    Ok(())
}
#[tauri::command]
pub async fn trust_connection(bridge: tauri::State<'_, Arc<Bridge>>) -> Result<(), String> {
    let bridge = bridge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        bridge.ensure_certificate()?;
        if bridge.verify_trust() {
            if let Ok(mut cache) = bridge.trust_cache.lock() {
                *cache = Some((Instant::now(), true));
            }
            return Ok(());
        }
        let home = std::env::var_os("HOME").ok_or("Cannot find your login keychain.")?;
        let keychain = PathBuf::from(home).join("Library/Keychains/login.keychain-db");
        let status = std::process::Command::new("/usr/bin/security")
            .args([
                "add-trusted-cert",
                "-r",
                "trustRoot",
                "-p",
                "ssl",
                "-s",
                "localhost.cts.md",
                "-k",
            ])
            .arg(keychain)
            .arg(bridge.certificate())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| "Cannot open macOS certificate setup.")?;
        if !status.success() {
            return Err(
                "macOS did not approve the local certificate. You can try setup again.".into(),
            );
        }
        if let Ok(mut cache) = bridge.trust_cache.lock() { *cache = None; }
        if !bridge.trusted() { return Err("The connection is not trusted yet. Complete the macOS prompt, then try setup again.".into()); }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn error(code: StatusCode, message: &str) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response.headers_mut().insert(
        "error",
        HeaderValue::from_str(message)
            .unwrap_or(HeaderValue::from_static("Login operation failed")),
    );
    response
}
async fn guard(request: Request<Body>, next: Next) -> Response {
    let origin = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let host = request.headers().get("host").and_then(|v| v.to_str().ok());
    // HTTP/2 carries :authority in the URI; HTTP/1.1 uses Host.
    let authority = request.uri().authority().map(|a| a.as_str());
    if host.or(authority) != Some("localhost.cts.md:18443")
        || host.is_some_and(|h| h != "localhost.cts.md:18443")
        || authority.is_some_and(|h| h != "localhost.cts.md:18443")
    {
        return error(StatusCode::FORBIDDEN, "Invalid local host");
    }
    // Origin is mandatory even for polling: never accept an arbitrary website or DNS rebinding request.
    if origin.as_deref() != Some(ORIGIN) && !(origin.is_none() && request.uri().path() == "/health")
    {
        return error(
            StatusCode::FORBIDDEN,
            "Only https://mpass.gov.md is allowed",
        );
    }
    if request.method() == "OPTIONS" {
        let method = request
            .headers()
            .get("access-control-request-method")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !["GET", "POST"].contains(&method) {
            return error(StatusCode::FORBIDDEN, "Method not allowed");
        }
        let headers = request
            .headers()
            .get("access-control-request-headers")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if headers.split(',').map(str::trim).any(|h| {
            !h.is_empty()
                && !h.eq_ignore_ascii_case("content-type")
                && !h.eq_ignore_ascii_case("accept")
        }) {
            return error(StatusCode::FORBIDDEN, "Headers not allowed");
        }
    }
    let mut response = if request.method() == "OPTIONS" {
        Response::builder().status(204).body(Body::empty()).unwrap()
    } else {
        next.run(request).await
    };
    let h = response.headers_mut();
    if origin.as_deref() == Some(ORIGIN) {
        h.insert(
            "access-control-allow-origin",
            HeaderValue::from_static(ORIGIN),
        );
        h.insert(
            "access-control-allow-methods",
            HeaderValue::from_static("GET, POST, OPTIONS"),
        );
        h.insert(
            "access-control-allow-headers",
            HeaderValue::from_static("Content-Type, Accept"),
        );
        h.insert(
            "access-control-expose-headers",
            HeaderValue::from_static("Location, error, sessionId"),
        );
        h.insert(
            "access-control-allow-private-network",
            HeaderValue::from_static("true"),
        );
    }
    h.insert("cache-control", HeaderValue::from_static("no-store"));
    h.insert("vary", HeaderValue::from_static("Origin"));
    response
}
async fn certificates(State(bridge): State<Arc<Bridge>>) -> Response {
    if bridge.scan_busy.swap(true, Ordering::SeqCst) {
        return error(
            StatusCode::TOO_MANY_REQUESTS,
            "Token scan already in progress",
        );
    }
    struct ScanGuard(Arc<Bridge>);
    impl Drop for ScanGuard {
        fn drop(&mut self) {
            self.0.scan_busy.store(false, Ordering::SeqCst);
        }
    }
    let _guard = ScanGuard(bridge.clone());
    if let Ok(inner) = bridge.inner.lock() {
        if inner.pending.as_ref().is_some_and(|p| p.signing) {
            return error(StatusCode::CONFLICT, "A login is already being signed");
        }
    }

    let probe = match tauri::async_runtime::spawn_blocking(crate::run_probe).await {
        Ok(Ok(p)) => p,
        _ => return error(StatusCode::SERVICE_UNAVAILABLE, "Cannot read the token"),
    };
    if probe["status"] == "error" {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "SafeNet could not read the token",
        );
    }
    let mut models = vec![];
    for token in probe["tokens"].as_array().into_iter().flatten() {
        if token["pinLocked"] == true {
            continue;
        }
        for cert in token["certificates"].as_array().into_iter().flatten() {
            if cert["valid"] != true {
                continue;
            }
            // Every identifier and DER value is obtained from the local token, never the browser.
            models.push(json!({"certificateId":cert["fingerprint"],"providerId":"PKCS11","label":cert["fingerprint"],"certificateName":cert["subject"],"certificateBase64":cert["der"],"subjectDN":cert["subject"],"issuerDN":cert["issuer"],"privateKeyPresent":true,"pinFinalTry":token["pinFinalTry"]}));
        }
    }
    if let Ok(mut inner) = bridge.inner.lock() {
        inner.certificates = models
            .iter()
            .map(|v| {
                (
                    v["certificateId"].as_str().unwrap_or_default().to_owned(),
                    v.clone(),
                )
            })
            .collect();
    }
    Json(json!({"certificateModel":models})).into_response()
}
use axum::response::IntoResponse;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SignInput {
    algorithm: String,
    certificate: Value,
    signature_type: String,
    content_type: String,
    sign_format: String,
    data: String,
}
fn validate_input(input: &SignInput) -> Result<Vec<u8>, String> {
    if input.algorithm != "SHA-1"
        || input.signature_type != "Embedded"
        || input.content_type != "Text"
        || input.sign_format != "XAdES-T"
    {
        return Err("Only the MPass login challenge is supported".into());
    }
    let challenge = B64.decode(&input.data).map_err(|_| "Invalid challenge")?;
    if challenge.len() != 20 {
        return Err("Invalid MPass challenge length".into());
    }
    Ok(challenge)
}
async fn sign(
    State(bridge): State<Arc<Bridge>>,
    headers: HeaderMap,
    Json(input): Json<SignInput>,
) -> Response {
    if !headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .starts_with("application/json")
    {
        return error(StatusCode::UNSUPPORTED_MEDIA_TYPE, "JSON required");
    }
    let challenge = match validate_input(&input) {
        Ok(c) => c,
        Err(e) => return error(StatusCode::BAD_REQUEST, &e),
    };
    let mut inner = match bridge.inner.lock() {
        Ok(i) => i,
        Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "Service unavailable"),
    };
    if inner.updating {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "App update in progress; try again shortly",
        );
    }
    if inner
        .pending
        .as_ref()
        .is_some_and(|p| p.created.elapsed() < REQUEST_TTL)
    {
        return error(
            StatusCode::CONFLICT,
            "Another login request is already open",
        );
    }
    let certificate = match input.certificate["certificateId"]
        .as_str()
        .and_then(|id| inner.certificates.get(id))
    {
        Some(c) => c.clone(),
        None => {
            return error(
                StatusCode::BAD_REQUEST,
                "Select a certificate again on MPass",
            )
        }
    };
    let id = uuid::Uuid::new_v4().to_string();
    inner.pending = Some(Operation {
        id: id.clone(),
        created: Instant::now(),
        certificate,
        challenge,
        signing: false,
        result: None,
    });
    inner.last_message = "MPass requested login. Approve it in this app.".into();
    if let Some(window) = bridge.app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let _ = window.set_focus();
    }
    // MPass expects an empty 201 body and then follows Location in jQuery's JSON failure callback.
    Response::builder()
        .status(201)
        .header("location", format!("{BASE}/sign/data/PKCS11/{id}/XAdES"))
        .header("sessionId", id)
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap()
}
async fn result(
    State(bridge): State<Arc<Bridge>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if uuid::Uuid::parse_str(&id).is_err() {
        return error(StatusCode::NOT_FOUND, "Unknown login request");
    }
    loop {
        {
            let mut inner = match bridge.inner.lock() {
                Ok(i) => i,
                Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "Service unavailable"),
            };
            match inner.pending.as_ref() {
                Some(p) if p.id == id => {
                    if p.created.elapsed() > REQUEST_TTL {
                        inner.pending = None;
                        bridge.release_attention();
                        return error(StatusCode::REQUEST_TIMEOUT, "Login request expired");
                    }
                    if p.result.is_some() {
                        let operation = inner.pending.take().unwrap();
                        return match operation.result.unwrap() {
                            Ok(xml) => Json(json!({"base64File":B64.encode(xml)})).into_response(),
                            Err(e) => error(StatusCode::BAD_REQUEST, &e),
                        };
                    }
                }
                _ => return error(StatusCode::NOT_FOUND, "Unknown or completed login request"),
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
#[tauri::command]
pub fn reject_login(id: String, bridge: tauri::State<'_, Arc<Bridge>>) -> Result<(), String> {
    let mut inner = bridge.inner.lock().map_err(|_| "Service unavailable.")?;
    let p = inner
        .pending
        .as_mut()
        .filter(|p| p.id == id && !p.signing)
        .ok_or("This request is no longer open.")?;
    p.result = Some(Err("Login cancelled by user".into()));
    inner.last_message = "Login cancelled.".into();
    bridge.release_attention();
    Ok(())
}
#[tauri::command]
pub async fn approve_login(
    id: String,
    pin: String,
    final_try_confirmed: bool,
    bridge: tauri::State<'_, Arc<Bridge>>,
) -> Result<(), String> {
    let pin = zeroize::Zeroizing::new(pin);
    if pin.is_empty() || pin.len() > 128 {
        return Err("Enter your token PIN.".into());
    }
    let bridge = bridge.inner().clone();
    let (certificate, challenge) = {
        let mut inner = bridge.inner.lock().map_err(|_| "Service unavailable.")?;
        let p = inner
            .pending
            .as_mut()
            .filter(|p| {
                p.id == id && !p.signing && p.result.is_none() && p.created.elapsed() < REQUEST_TTL
            })
            .ok_or("This request expired or is no longer open.")?;
        if p.certificate["pinFinalTry"] == true && !final_try_confirmed {
            return Err("Confirm that you understand this is the final PIN attempt.".into());
        }
        p.signing = true;
        (p.certificate.clone(), p.challenge.clone())
    };
    let signing_id = id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let der = B64
            .decode(
                certificate["certificateBase64"]
                    .as_str()
                    .ok_or("Missing certificate.")?,
            )
            .map_err(|_| "Invalid certificate.")?;
        let prepared = crate::xades::Prepared::new(&der, &challenge, &signing_id)?;
        let signature = crate::run_sign(
            certificate["certificateId"]
                .as_str()
                .ok_or("Missing certificate ID.")?,
            prepared.signed_info.as_bytes(),
            &pin,
            final_try_confirmed,
        )?;
        prepared.finish(&signature)
    })
    .await
    .unwrap_or_else(|_| {
        Err("The signing task stopped unexpectedly. Your PIN was not retried.".into())
    });
    let success = result.is_ok();
    let message = result.as_ref().err().cloned();
    {
        let mut inner = bridge.inner.lock().map_err(|_| "Service unavailable.")?;
        if let Some(p) = inner.pending.as_mut().filter(|p| p.id == id && p.signing) {
            p.result = Some(result);
        }
        inner.last_message = if success {
            "Login signature ready for MPass. Continue in your browser.".into()
        } else {
            format!(
                "Login failed: {}. Start again on MPass; the PIN is not retried automatically.",
                message.clone().unwrap_or_default()
            )
        };
    }
    bridge.release_attention();
    if let Some(message) = message {
        return Err(message);
    }
    Ok(())
}
pub fn initialize(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_data = app.path().app_data_dir()?;
    let directory = app_data.join("connection");
    if let Some(parent) = app_data.parent() {
        let previous = parent.join("dev.local.moldsignbridge.preview/connection");
        crate::connection_migration::migrate_connection(&previous, &directory)?;
    }
    app.manage(Arc::new(Bridge::new(directory, app.handle().clone())));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_mpass_challenge_shape_is_accepted() {
        let mut i = SignInput {
            algorithm: "SHA-1".into(),
            certificate: json!({}),
            signature_type: "Embedded".into(),
            content_type: "Text".into(),
            sign_format: "XAdES-T".into(),
            data: B64.encode([7; 20]),
        };
        assert!(validate_input(&i).is_ok());
        i.data = B64.encode(b"sign this arbitrary document");
        assert!(validate_input(&i).is_err());
        i.data = B64.encode([7; 20]);
        i.sign_format = "PAdES".into();
        assert!(validate_input(&i).is_err());
    }
}

#[cfg(test)]
mod origin_tests {
    use super::*;
    use tower::ServiceExt;
    async fn request(origin: Option<&str>, host: &str, method: &str, preflight: bool) -> Response {
        let router = Router::new()
            .route("/certificates", get(|| async { "test" }))
            .layer(middleware::from_fn(guard));
        let mut req = Request::builder()
            .uri("/certificates")
            .method(method)
            .header("host", host);
        if let Some(origin) = origin {
            req = req.header("origin", origin);
        }
        if preflight {
            req = req
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "content-type")
                .header("access-control-request-private-network", "true");
        }
        router
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn denies_untrusted_sites_and_missing_origins() {
        for origin in [
            None,
            Some("null"),
            Some("https://evil.example"),
            Some("https://mpass.gov.md.evil.example"),
        ] {
            let r = request(origin, "localhost.cts.md:18443", "GET", false).await;
            assert_eq!(r.status(), StatusCode::FORBIDDEN);
            assert!(!r.headers().contains_key("access-control-allow-origin"));
        }
        assert_eq!(
            request(Some(ORIGIN), "evil.example:18443", "GET", false)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    #[tokio::test]
    async fn mpass_preflight_exposes_location_and_allows_local_network() {
        let r = request(Some(ORIGIN), "localhost.cts.md:18443", "OPTIONS", true).await;
        assert_eq!(r.status(), StatusCode::NO_CONTENT);
        assert_eq!(r.headers()["access-control-allow-origin"], ORIGIN);
        assert_eq!(r.headers()["access-control-allow-private-network"], "true");
        assert!(r.headers()["access-control-expose-headers"]
            .to_str()
            .unwrap()
            .contains("Location"));
    }
}

#[tauri::command]
pub fn open_mpass() -> Result<(), String> {
    std::process::Command::new("/usr/bin/open")
        .arg("https://mpass.gov.md/login")
        .spawn()
        .map_err(|_| "Cannot open your browser.")?;
    Ok(())
}

#[cfg(test)]
mod http2_tests {
    use super::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn validates_http2_authority_without_host() {
        for (authority, expected) in [
            ("localhost.cts.md:18443", StatusCode::OK),
            ("evil.example:18443", StatusCode::FORBIDDEN),
        ] {
            let router = Router::new()
                .route("/certificates", get(|| async { "test" }))
                .layer(middleware::from_fn(guard));
            let req = Request::builder()
                .version(axum::http::Version::HTTP_2)
                .uri(format!("https://{authority}/certificates"))
                .header("origin", ORIGIN)
                .body(Body::empty())
                .unwrap();
            let response = router.oneshot(req).await.unwrap();
            assert_eq!(response.status(), expected);
        }
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;

    fn idle() -> Inner {
        Inner {
            updating: false,
            certificates: HashMap::new(),
            pending: None,
            last_message: String::new(),
        }
    }
    #[test]
    fn update_reservation_is_exclusive_and_releasable() {
        let mut inner = idle();
        assert!(reserve_update(&mut inner).is_ok());
        assert!(inner.updating);
        assert!(reserve_update(&mut inner).is_err());
        inner.updating = false;
        assert!(reserve_update(&mut inner).is_ok());
    }
    #[test]
    fn pending_and_signing_logins_block_updates() {
        for signing in [false, true] {
            let mut inner = idle();
            inner.pending = Some(Operation {
                id: "test".into(),
                created: Instant::now(),
                certificate: Value::Null,
                challenge: vec![],
                signing,
                result: None,
            });
            assert!(reserve_update(&mut inner).is_err());
            assert!(!inner.updating);
            inner.pending = None;
            assert!(reserve_update(&mut inner).is_ok());
        }
    }
}
