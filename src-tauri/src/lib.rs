// =====================================================================
//  src-tauri/src/lib.rs
//  Sostituisci il contenuto del lib.rs generato da `npm create tauri-app`.
//  main.rs resta com'e' (chiama run()).
//
//  Comportamento aggiornamenti (come richiesto):
//   avvio -> controlla su GitHub -> se c'e' una versione nuova CHIEDE
//   "aggiornare ora?" -> se SI': scarica, installa e riavvia gia' aggiornato.
// =====================================================================

use std::env;
use std::path::PathBuf;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;

/// Cartella dei dati: lavori-da-fare.md, posta-nuova.md, leggi-posta.ps1.
/// Default %USERPROFILE%\Desktop\Tasks, sovrascrivibile con LAVORI_DIR.
fn data_dir() -> PathBuf {
    if let Ok(custom) = env::var("LAVORI_DIR") {
        return PathBuf::from(custom);
    }
    let profile = env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    PathBuf::from(profile).join("Desktop").join("Tasks")
}

#[tauri::command]
fn read_tasks() -> Result<String, String> {
    let path = data_dir().join("lavori-da-fare.md");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("Impossibile leggere {}: {}", path.display(), e))
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Avvia leggi-posta.ps1 (Outlook classico) in background, finestra nascosta.
fn avvia_lettura_posta() {
    let script = data_dir().join("leggi-posta.ps1");
    if !script.exists() {
        eprintln!("leggi-posta.ps1 non trovato in {}", script.display());
        return;
    }
    let _ = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(script)
        .spawn();
}

/// Controllo aggiornamenti con richiesta di conferma.
fn controlla_aggiornamenti(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let updater = match app.updater() {
            Ok(u) => u,
            Err(_) => return,
        };
        // Nessuna release ancora pubblicata -> check fallisce: normale, esci.
        let update = match updater.check().await {
            Ok(Some(u)) => u,
            _ => return,
        };

        let vecchia = app.package_info().version.to_string();
        let messaggio = format!(
            "È disponibile la versione {} (hai la {}).\n\nVuoi aggiornare ora?\nL'app scaricherà, installerà e si riavvierà da sola.",
            update.version, vecchia
        );

        let vuole = app
            .dialog()
            .message(messaggio)
            .title("Aggiornamento disponibile")
            .buttons(MessageDialogButtons::YesNo)
            .blocking_show();

        if vuole {
            if update.download_and_install(|_, _| {}, || {}).await.is_ok() {
                app.restart();
            } else {
                app.dialog()
                    .message("Aggiornamento non riuscito. Riprova piu' tardi.")
                    .title("Errore aggiornamento")
                    .blocking_show();
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            avvia_lettura_posta();
            controlla_aggiornamenti(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_tasks, app_version])
        .run(tauri::generate_context!())
        .expect("errore nell'avvio dell'app Tauri");
}
