use std::path::PathBuf;

use crate::api::Result;
use theseus::prelude::*;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("profile_create")
        .invoke_handler(tauri::generate_handler![
            profile_create_empty,
            profile_create,
        ])
        .build()
}

// Generic basic profile creation tool.
// Creates an essentially empty dummy profile with profile_create
#[tauri::command]
pub async fn profile_create_empty() -> Result<String> {
    let res = profile_create::profile_create_empty().await?;
    let res = res.to_string_lossy().to_string();
    Ok(res)
}

// Creates a profile at  the given filepath and adds it to the in-memory state
// invoke('profile_add',profile)
#[tauri::command]
pub async fn profile_create(
    name: String,         // the name of the profile, and relative path
    game_version: String, // the game version of the profile
    modloader: ModLoader, // the modloader to use
    loader_version: Option<String>, // the modloader version to use, set to "latest", "stable", or the ID of your chosen loader
    icon: Option<String>,           // the icon for the profile
) -> Result<String> {
    let res = profile_create::profile_create(
        name,
        game_version,
        modloader,
        loader_version,
        icon.map(PathBuf::from),
        None,
        None,
        None,
    )
    .await?;
    let res = res.to_string_lossy().to_string();
    Ok(res)
}
