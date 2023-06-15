use std::path::PathBuf;

use crate::api::Result;
use tauri::plugin::TauriPlugin;
use theseus::prelude::JavaVersion;
use theseus::prelude::*;

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("jre")
        .invoke_handler(tauri::generate_handler![
            jre_get_all_jre,
            jre_get_complex,
            jre_get_complex2,
            jre_get_complex3,
            jre_get_complex4,
            jre_find_jre_8_jres,
            jre_find_jre_17_jres,
            jre_find_jre_18plus_jres,
            // jre_autodetect_java_globals,
            // jre_validate_globals,
            // jre_get_jre,
            // jre_auto_install_java,
            // jre_get_max_memory,
        ])
        .build()
}

#[tauri::command]
#[inline(always)]
pub async fn jre_get_complex() -> Result<Vec<JavaVersion>> {
    Ok(jre::get_complex_jre_dbg().await?)
}

#[tauri::command]
#[inline(always)]
pub async fn jre_get_complex2() -> Result<Vec<JavaVersion>> {
    Ok(jre::get_complex_jre_dbg_2().await?)
}

#[tauri::command]
#[inline(always)]
pub async fn jre_get_complex3() -> Result<Vec<JavaVersion>> {
    Ok(jre::get_complex_jre_dbg_3().await?)
}

#[tauri::command]
#[inline(always)]
pub async fn jre_get_complex4() -> Result<Vec<JavaVersion>> {
    Ok(jre::get_complex_jre_dbg_4().await?)
}





/// Get all JREs that exist on the system
#[tauri::command]
#[inline(always)]
pub async fn jre_get_all_jre() -> Result<Vec<JavaVersion>> {
    Ok(jre::get_all_jre().await?)
}

// Finds the isntallation of Java 7, if it exists
#[tauri::command]
#[inline(always)]
pub async fn jre_find_jre_8_jres() -> Result<Vec<JavaVersion>> {
    Ok(jre::find_java8_jres().await?)
}

// finds the installation of Java 17, if it exists
#[tauri::command]
#[inline(always)]
pub async fn jre_find_jre_17_jres() -> Result<Vec<JavaVersion>> {
    Ok(jre::find_java17_jres().await?)
}

// Finds the highest version of Java 18+, if it exists
#[tauri::command]
#[inline(always)]
pub async fn jre_find_jre_18plus_jres() -> Result<Vec<JavaVersion>> {
    Ok(jre::find_java18plus_jres().await?)
}

// Autodetect Java globals, by searching the users computer.
// Returns a *NEW* JavaGlobals that can be put into Settings
// #[tauri::command]

pub async fn jre_autodetect_java_globals() -> Result<JavaGlobals> {
    Ok(jre::autodetect_java_globals().await?)
}

// Validates java globals, by checking if the paths exist
// If false, recommend to direct them to reassign, or to re-guess
#[tauri::command]
pub async fn jre_validate_globals() -> Result<bool> {
    Ok(jre::validate_globals().await?)
}

// Validates JRE at a given path
// Returns None if the path is not a valid JRE
// #[tauri::command]
pub async fn jre_get_jre(path: String) -> Result<Option<JavaVersion>> {
    jre::check_jre(PathBuf::from(path))
        .await
        .map_err(|e| e.into())
}

// Auto installs java for the given java version
// #[tauri::command]
pub async fn jre_auto_install_java(java_version: u32) -> Result<String> {
    let res = jre::auto_install_java(java_version).await?;
    let res = res.to_string_lossy().to_string();
    Ok(res)
}

// Gets the maximum memory a system has available.
// #[tauri::command]
pub async fn jre_get_max_memory() -> Result<u64> {
    Ok(jre::get_max_memory().await?)
}
