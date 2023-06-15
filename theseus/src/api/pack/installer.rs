use crate::util::fetch::write;
use crate::{
    event::{
        emit::{
            emit_loading, init_or_edit_loading, loading_try_for_each_concurrent,
        },
        LoadingBarId,
    },
    pack::install_from::{EnvType, PackFile, PackFileHash},
    prelude::ModLoader,
    state::{LinkedData, ProfileInstallStage, SideType},
    util::fetch::fetch_mirrors,
    LoadingBarType, State,
};
use async_zip::{tokio::read::seek::ZipFileReader, ZipEntry};
use bytes::Bytes;
use std::io::Cursor;
use std::path::{Component, PathBuf};

use super::install_from::{PackDependency, PackFormat};

type Zip<'a> = ZipFileReader<Cursor<&'a Bytes>>;

#[tracing::instrument(skip(file))]
#[theseus_macros::debug_pin]
pub async fn install_pack(
    file: bytes::Bytes,
    icon: Option<PathBuf>,
    override_title: Option<String>,
    project_id: Option<String>,
    version_id: Option<String>,
    existing_loading_bar: Option<LoadingBarId>,
    profile: PathBuf,
) -> crate::Result<PathBuf> {
    match install_helper(
        file,
        icon,
        override_title,
        project_id,
        version_id,
        existing_loading_bar,
        profile.clone(),
    )
    .await
    {
        Ok(profile) => Ok(profile),
        Err(err) => {
            let _ = crate::api::profile::remove(&profile).await;
            Err(err)
        }
    }
}

pub async fn install_helper(
    file: bytes::Bytes,
    icon: Option<PathBuf>,
    override_title: Option<String>,
    project_id: Option<String>,
    version_id: Option<String>,
    existing_loading_bar: Option<LoadingBarId>,
    profile: PathBuf,
) -> crate::Result<PathBuf> {
    let reader: Cursor<&bytes::Bytes> = Cursor::new(&file);

    // Create zip reader around file
    let mut zip_reader = ZipFileReader::new(reader).await.map_err(|_| {
        crate::Error::from(crate::ErrorKind::InputError(
            "Failed to read input modpack zip".to_string(),
        ))
    })?;

    // Extract index of modrinth.index.json
    let zip_index_option = zip_reader
        .file()
        .entries()
        .iter()
        .position(|f| f.entry().filename() == "modrinth.index.json");

    if let Some(zip_index) = zip_index_option {
        install_zip_entry(
            &mut zip_reader,
            zip_index,
            icon,
            override_title,
            project_id,
            version_id,
            existing_loading_bar,
            profile,
        )
        .await
    } else {
        Err(crate::Error::from(crate::ErrorKind::InputError(
            "No pack manifest found in mrpack".to_string(),
        )))
    }
}

async fn install_zip_entry<'a>(
    zip_reader: &mut Zip<'a>,
    zip_index: usize,
    icon: Option<PathBuf>,
    override_title: Option<String>,
    project_id: Option<String>,
    version_id: Option<String>,
    existing_loading_bar: Option<LoadingBarId>,
    profile: PathBuf,
) -> crate::Result<PathBuf> {
    let _state = &State::get().await?;
    let mut manifest = String::new();
    let entry = zip_reader
        .file()
        .entries()
        .get(zip_index)
        .unwrap()
        .entry()
        .clone();
    let mut reader = zip_reader.entry(zip_index).await?;
    reader.read_to_string_checked(&mut manifest, &entry).await?;

    let pack: PackFormat = serde_json::from_str(&manifest)?;

    if &*pack.game != "minecraft" {
        return Err(crate::ErrorKind::InputError(
            "Pack does not support Minecraft".to_string(),
        )
        .into());
    }

    let mut game_version = None;
    let mut mod_loader = None;
    let mut loader_version = None;
    for (key, value) in &pack.dependencies {
        match key {
            PackDependency::Forge => {
                mod_loader = Some(ModLoader::Forge);
                loader_version = Some(value);
            }
            PackDependency::FabricLoader => {
                mod_loader = Some(ModLoader::Fabric);
                loader_version = Some(value);
            }
            PackDependency::QuiltLoader => {
                mod_loader = Some(ModLoader::Quilt);
                loader_version = Some(value);
            }
            PackDependency::Minecraft => game_version = Some(value),
        }
    }

    let game_version = if let Some(game_version) = game_version {
        game_version
    } else {
        return Err(crate::ErrorKind::InputError(
            "Pack did not specify Minecraft version".to_string(),
        )
        .into());
    };

    let loader_version = crate::profile_create::get_loader_version_from_loader(
        game_version.clone(),
        mod_loader.unwrap_or(ModLoader::Vanilla),
        loader_version.cloned(),
    )
    .await?;
    crate::api::profile::edit(&profile, |prof| {
        prof.metadata.name =
            override_title.clone().unwrap_or_else(|| pack.name.clone());
        prof.install_stage = ProfileInstallStage::PackInstalling;
        prof.metadata.linked_data = Some(LinkedData {
            project_id: project_id.clone(),
            version_id: version_id.clone(),
        });
        prof.metadata.icon = icon.clone();
        prof.metadata.game_version = game_version.clone();
        prof.metadata.loader_version = loader_version.clone();
        prof.metadata.loader = mod_loader.unwrap_or(ModLoader::Vanilla);

        async { Ok(()) }
    })
    .await?;
    State::sync().await?;

    let profile = profile.clone();

    match install_pack_file(
        zip_reader,
        icon,
        project_id,
        version_id,
        existing_loading_bar,
        pack,
        profile.clone(),
    )
    .await
    {
        Ok(profile) => Ok(profile),
        Err(err) => {
            let _ = crate::api::profile::remove(&profile).await;

            Err(err)
        }
    }
}

async fn install_pack_file<'a>(
    zip_reader: &mut Zip<'a>,
    icon: Option<PathBuf>,
    project_id: Option<String>,
    version_id: Option<String>,
    existing_loading_bar: Option<LoadingBarId>,
    pack: PackFormat,
    profile: PathBuf,
) -> crate::Result<PathBuf> {
    let loading_bar = init_or_edit_loading(
        existing_loading_bar,
        LoadingBarType::PackDownload {
            profile_path: profile.clone(),
            pack_name: pack.name.clone(),
            icon,
            pack_id: project_id,
            pack_version: version_id,
        },
        100.0,
        "Downloading modpack",
    )
    .await?;

    let num_files = pack.files.len();
    use futures::StreamExt;
    loading_try_for_each_concurrent(
        futures::stream::iter(pack.files.into_iter())
            .map(Ok::<PackFile, crate::Error>),
        None,
        Some(&loading_bar),
        70.0,
        num_files,
        None,
        |project| {
            let profile = profile.clone();
            install_zip_file(project, profile)
        },
    )
    .await?;

    emit_loading(&loading_bar, 0.0, Some("Extracting overrides")).await?;

    let mut total_len = 0;

    for index in 0..zip_reader.file().entries().len() {
        let file = zip_reader.file().entries().get(index).unwrap().entry();

        if (file.filename().starts_with("overrides")
            || file.filename().starts_with("client_overrides"))
            && !file.filename().ends_with('/')
        {
            total_len += 1;
        }
    }

    for index in 0..zip_reader.file().entries().len() {
        let file = zip_reader
            .file()
            .entries()
            .get(index)
            .unwrap()
            .entry()
            .clone();

        install_overrides(
            file,
            zip_reader,
            index,
            total_len,
            profile.clone(),
            &loading_bar,
        )
        .await?;
    }

    if let Some(profile_val) = crate::api::profile::get(&profile, None).await? {
        crate::launcher::install_minecraft(&profile_val, Some(loading_bar))
            .await?;
    }

    Ok::<PathBuf, crate::Error>(profile.clone())
}

pub async fn install_zip_file(
    project: PackFile,
    profile: PathBuf,
) -> crate::Result<()> {
    let state = &State::get().await?;
    //TODO: Future update: prompt user for optional files in a modpack
    if let Some(env) = project.env {
        if env
            .get(&EnvType::Client)
            .map(|x| x == &SideType::Unsupported)
            .unwrap_or(false)
        {
            return Ok(());
        }
    }

    let file = fetch_mirrors(
        &project
            .downloads
            .iter()
            .map(|x| &**x)
            .collect::<Vec<&str>>(),
        project.hashes.get(&PackFileHash::Sha1).map(|x| &**x),
        &state.fetch_semaphore,
    )
    .await?;

    let path = std::path::Path::new(&project.path).components().next();
    if let Some(path) = path {
        match path {
            Component::CurDir | Component::Normal(_) => {
                let path = profile.join(project.path);
                write(&path, &file, &state.io_semaphore).await?;
            }
            _ => {}
        };
    }
    Ok(())
}

async fn install_overrides<'a>(
    file: ZipEntry,
    zip_reader: &mut Zip<'a>,
    index: usize,
    total_len: usize,
    profile: PathBuf,
    loading_bar: &LoadingBarId,
) -> crate::Result<()> {
    let state = &State::get().await?;
    let file_path = PathBuf::from(file.filename());
    if (file.filename().starts_with("overrides")
        || file.filename().starts_with("client_overrides"))
        && !file.filename().ends_with('/')
    {
        // Reads the file into the 'content' variable
        let mut content = Vec::new();
        let mut reader = zip_reader.entry(index).await?;
        reader.read_to_end_checked(&mut content, &file).await?;

        let mut new_path = PathBuf::new();
        let components = file_path.components().skip(1);

        for component in components {
            new_path.push(component);
        }

        if new_path.file_name().is_some() {
            write(&profile.join(new_path), &content, &state.io_semaphore)
                .await?;
        }

        emit_loading(
            loading_bar,
            30.0 / total_len as f64,
            Some(&format!("Extracting override {}/{}", index, total_len)),
        )
        .await?;
    }
    Ok(())
}
