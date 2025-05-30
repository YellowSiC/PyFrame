// Copyright 2025-2030 PyFrame Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod image_utils;
#[cfg(target_os = "windows")]
mod win_utils;

use anyhow::{Ok, Result};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tao::window::Icon;

use crate::{
    lock,
    utils::{arc, arc_mut, ArcMut},
};

type IconCache = HashMap<String, Icon>;

pub trait ResourceManager: std::fmt::Debug + Send + Sync {
    #[allow(dead_code)]
    fn exists(&self, path: &str) -> bool;
    #[allow(dead_code)]
    fn load(&self, path: &str) -> Result<Vec<u8>>;
    #[allow(dead_code)]
    fn extract(&self, from: &str, to: &Path) -> Result<()>;
    #[allow(dead_code)]
    fn load_icon(&self, path: &str) -> Result<Icon>;

    // ➜ NEU: Direkt aus Bytes laden
    #[allow(dead_code)]
    fn load_icon_from_bytes(&self, data: &[u8]) -> Result<Icon>;
}

#[derive(Debug)]
pub struct FileSystemResource {
    root_dir: PathBuf,
    icon_cache: ArcMut<IconCache>,
}

impl FileSystemResource {
    #[allow(dead_code)]
    pub fn new(root_dir: &Path) -> Result<Arc<FileSystemResource>> {
        root_dir
            .exists()
            .then(|| root_dir.is_dir())
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?;
        Ok(arc(FileSystemResource {
            root_dir: root_dir.to_path_buf(),
            icon_cache: arc_mut(HashMap::new()),
        }))
    }
}

impl ResourceManager for FileSystemResource {
    fn exists(&self, path: &str) -> bool {
        let path = self.root_dir.join(path);
        path.exists() && path.is_file()
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        Ok(std::fs::read(self.root_dir.join(path))?)
    }

    fn extract(&self, from: &str, to: &Path) -> Result<()> {
        fs_extra::file::copy(self.root_dir.join(from), to, &fs_extra::file::CopyOptions::new())?;
        Ok(())
    }

    fn load_icon_from_bytes(&self, data: &[u8]) -> Result<Icon> {
        // ➜ einfach die bestehende Hilfsmethode nutzen
        let icon = image_utils::png_to_icon(data)?;
        Ok(icon)
    }
    fn load_icon(&self, path: &str) -> Result<Icon> {
        let mut cache = lock!(self.icon_cache)?;
        let icon = cache.get(path);
        match icon {
            Some(icon) => Ok(icon.clone()),
            None => {
                let data = self.load(path)?;
                if path.ends_with("png") {
                    let icon = image_utils::png_to_icon(&data)?;
                    cache.insert(path.to_string(), icon.clone());
                    Ok(icon)
                } else {
                    Err(anyhow::anyhow!("Unsupported icon format."))
                }
            }
        }
    }
}

pub struct AppResourceManager {
    indexes: HashMap<String, (usize, usize)>,
    data: Vec<u8>,
    icon_cache: Mutex<IconCache>,
}

impl std::fmt::Debug for AppResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacOSAppResourceManager")
            .field("indexes", &self.indexes)
            .field("data", &"Vec<u8>")
            .finish()
    }
}

impl AppResourceManager {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn new() -> Result<Arc<AppResourceManager>> {
        let resources_dir = std::env::current_exe()?
            .parent()
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?
            .join("../Resources/");
        let indexes_data = std::fs::read(resources_dir.join("RESOURCE_INDEXES"))?;
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        let compressed_data = std::fs::read(resources_dir.join("RESOURCE_DATA"))?;
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        Ok(arc(AppResourceManager {
            indexes,
            data,
            icon_cache: Mutex::new(HashMap::new()),
        }))
    }

    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn new() -> Result<Arc<AppResourceManager>> {
        use win_utils::load_resource;
        let indexes_data = load_resource("RESOURCE_INDEXES")?;
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        let compressed_data = load_resource("RESOURCE_DATA")?;
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        Ok(arc(AppResourceManager {
            indexes,
            data,
            icon_cache: Mutex::new(HashMap::new()),
        }))
    }
}

impl ResourceManager for AppResourceManager {
    fn exists(&self, path: &str) -> bool {
        self.indexes.contains_key(path)
    }

    fn load_icon_from_bytes(&self, data: &[u8]) -> Result<Icon> {
        let icon = image_utils::png_to_icon(data)?;
        Ok(icon)
    }
    fn load(&self, path: &str) -> Result<Vec<u8>> {
        let (offset, length) = *self.indexes.get(path).ok_or(anyhow::anyhow!("File not found."))?;
        Ok(self.data[offset..(offset + length)].to_vec())
    }

    fn extract(&self, from: &str, to: &Path) -> Result<()> {
        let content = self.load(from)?;
        std::fs::write(to, content)?;
        Ok(())
    }

    fn load_icon(&self, path: &str) -> Result<Icon> {
        let mut cache = lock!(self.icon_cache)?;
        let icon = cache.get(path);
        match icon {
            Some(icon) => Ok(icon.clone()),
            None => {
                let data = self.load(path)?;
                if path.ends_with("png") {
                    let icon = image_utils::png_to_icon(&data)?;
                    cache.insert(path.to_string(), icon.clone());
                    Ok(icon)
                } else {
                    Err(anyhow::anyhow!("Unsupported icon format."))
                }
            }
        }
    }
}
