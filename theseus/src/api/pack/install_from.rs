use crate::config::MODRINTH_API_URL;
use crate::data::ModLoader;
use crate::event::emit::{emit_loading, init_loading};
use crate::event::{LoadingBarType, LoadingBarId};
use crate::state::{LinkedData, ModrinthProject, ModrinthVersion, SideType};
use crate::util::fetch::{
    fetch, fetch_advanced, fetch_json, write_cached_icon,
};
use crate::State;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

use super::installer::install_pack;

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackFormat {
    pub game: String,
    pub format_version: i32,
    pub version_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<PackFile>,
    pub dependencies: HashMap<PackDependency, String>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub path: String,
    pub hashes: HashMap<PackFileHash, String>,
    pub env: Option<HashMap<EnvType, SideType>>,
    pub downloads: Vec<String>,
    pub file_size: u32,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase", from = "String")]
pub enum PackFileHash {
    Sha1,
    Sha512,
    Unknown(String),
}

impl From<String> for PackFileHash {
    fn from(s: String) -> Self {
        return match s.as_str() {
            "sha1" => PackFileHash::Sha1,
            "sha512" => PackFileHash::Sha512,
            _ => PackFileHash::Unknown(s),
        };
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum EnvType {
    Client,
    Server,
}

#[derive(Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PackDependency {
    Forge,
    FabricLoader,
    QuiltLoader,
    Minecraft,
}

#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn install_pack_from_version_id(
    project_id: String,
    version_id: String,
    title: String,
    icon_url: Option<String>,
) -> crate::Result<PathBuf> {
    let state: std::sync::Arc<State> = State::get().await?;

    let profile = crate::api::profile_create::profile_create(
        title.clone(),
        "1.19.4".to_string(),
        ModLoader::Vanilla,
        None,
        None,
        icon_url.clone(),
        Some(LinkedData {
            project_id: Some(project_id),
            version_id: Some(version_id.clone()),
        }),
        Some(true),
    )
    .await?;

    let loading_bar = init_loading(
        LoadingBarType::PackFileDownload {
            profile_path: profile.clone(),
            pack_name: title,
            icon: icon_url,
            pack_version: version_id.clone(),
        },
        100.0,
        "Downloading pack file",
    )
    .await?;

    emit_loading(&loading_bar, 0.0, Some("Fetching version")).await?;
    let version: ModrinthVersion = fetch_json(
        Method::GET,
        &format!("{}version/{}", MODRINTH_API_URL, version_id),
        None,
        None,
        &state.fetch_semaphore,
    )
    .await?;
    emit_loading(&loading_bar, 10.0, None).await?;

    let (url, hash) =
        if let Some(file) = version.files.iter().find(|x| x.primary) {
            Some((file.url.clone(), file.hashes.get("sha1")))
        } else {
            version
                .files
                .first()
                .map(|file| (file.url.clone(), file.hashes.get("sha1")))
        }
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "Specified version has no files".to_string(),
            )
        })?;

    let file = fetch_advanced(
        Method::GET,
        &url,
        hash.map(|x| &**x),
        None,
        None,
        Some((&loading_bar, 70.0)),
        &state.fetch_semaphore,
    )
    .await?;
    emit_loading(&loading_bar, 0.0, Some("Fetching project metadata")).await?;

    let project: ModrinthProject = fetch_json(
        Method::GET,
        &format!("{}project/{}", MODRINTH_API_URL, version.project_id),
        None,
        None,
        &state.fetch_semaphore,
    )
    .await?;

    emit_loading(&loading_bar, 10.0, Some("Retrieving icon")).await?;
    let icon = get_icon(&project).await?;
    emit_loading(&loading_bar, 10.0, None).await?;

    install_pack(
        file,
        icon,
        Some(project.title),
        Some(version.project_id),
        Some(version.id),
        Some(loading_bar),
        profile,
    )
    .await
}

async fn get_icon(project: &ModrinthProject) -> crate::Result<Option<PathBuf>> {
   Ok(if let Some(icon_url) = &project.icon_url {
        let state = State::get().await?;
        let icon_bytes = fetch(&icon_url, None, &state.fetch_semaphore).await?;

        let filename = icon_url.rsplit('/').next();

        if let Some(filename) = filename {
            Some(
                write_cached_icon(
                    filename,
                    &state.directories.caches_dir(),
                    icon_bytes,
                    &state.io_semaphore,
                )
                .await?,
            )
        } else {
            None
        }
    } else {
        None
    })
}

#[tracing::instrument]
#[theseus_macros::debug_pin]
pub async fn install_pack_from_file(path: PathBuf) -> crate::Result<PathBuf> {
    let file: Vec<u8> = fs::read(&path).await?;

    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let profile = crate::api::profile_create::profile_create(
        file_name,
        "1.19.4".to_string(),
        ModLoader::Vanilla,
        None,
        None,
        None,
        None,
        Some(true),
    )
    .await?;

    install_pack(
        bytes::Bytes::from(file),
        None,
        None,
        None,
        None,
        None,
        profile,
    )
    .await
}
