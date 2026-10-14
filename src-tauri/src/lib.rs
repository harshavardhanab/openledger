use ledger_store::{LedgerService, write_atomic};
use serde_json::{Value, json};
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn ledger_request(
    command: String,
    payload: Value,
    service: State<'_, Arc<LedgerService>>,
) -> Result<Value, String> {
    service.request(&command, payload)
}
#[tauri::command]
async fn save_backup(
    app: tauri::AppHandle,
    service: State<'_, Arc<LedgerService>>,
) -> Result<Value, String> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move||{
  let backup=service.request("backup",json!({}))?;
  let selection=app.dialog().file().add_filter("Open Ledger backup",&["json"]).set_file_name("open-ledger-backup.json").blocking_save_file();
  let Some(file)=selection else{return Ok(json!({"saved":false}));};
  let path=file.into_path().map_err(|e|e.to_string())?;
  write_atomic(&path,backup.as_str().ok_or("Invalid backup response")?).map_err(|e|e.to_string())?;
  Ok(json!({"saved":true,"filename":path.file_name().map(|s|s.to_string_lossy().to_string())}))
 }).await.map_err(|e|e.to_string())?
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            app.manage(Arc::new(LedgerService::open(
                &directory.join("open-ledger.sqlite"),
            )?));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ledger_request, save_backup])
        .run(tauri::generate_context!())
        .expect("Open Ledger could not start");
}
