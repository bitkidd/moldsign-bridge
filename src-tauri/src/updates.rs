use crate::bridge::Bridge;
use std::{sync::Arc, time::Duration};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct UpdateState(Mutex<Option<Update>>);

#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, UpdateState>,
) -> Result<Option<String>, String> {
    let mut saved = state.0.lock().await;
    *saved = None;
    let update = app.updater_builder().timeout(Duration::from_secs(30)).build()
        .map_err(|e| e.to_string())?.check().await
        .map_err(|e| format!("Could not check for updates. The Alpha release channel may not be published yet. {e}"))?;
    let version = update.as_ref().map(|u| u.version.clone());
    *saved = update;
    Ok(version)
}

#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, UpdateState>,
    bridge: tauri::State<'_, Arc<Bridge>>,
) -> Result<(), String> {
    let mut saved = state.0.lock().await;
    let update = saved.as_ref().ok_or("Check for updates first.")?;
    bridge.begin_update()?;
    let result = update.download_and_install(|_, _| {}, || {}).await;
    if let Err(error) = result {
        bridge.end_update();
        return Err(format!("The update could not be installed. {error}"));
    }
    *saved = None;
    app.restart();
}
