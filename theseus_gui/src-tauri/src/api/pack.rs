use crate::api::Result;
use std::path::{Path};
use theseus::prelude::*;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("pack")
        .invoke_handler(tauri::generate_handler![
            pack_install_version_id,
            pack_install_file,
        ])
        .build()
}

#[tauri::command]
pub async fn pack_install_version_id(
    project_id: String,
    version_id: String,
    pack_title: String,
    pack_icon: Option<String>,
) -> Result<String> {
    let res: String = install_pack_from_version_id(
        project_id, version_id, pack_title, pack_icon,
    )
    .await?;
    Ok(res)
}


#[tauri::command]
pub async fn pack_install_file(path: String) -> Result<String> {
    let res = install_pack_from_file(path).await?;
    Ok(res)
}


pub async fn install_pack_from_file(path: String) -> Result<String> {
    panic!("hello");
}
pub async fn install_pack_from_version_id(
    project_id: String,
    version_id: String,
    pack_title: String,
    pack_icon: Option<String>,
) -> Result<String> {
    panic!("hel2lo");

}