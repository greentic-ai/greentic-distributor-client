use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use greentic_distributor_client::{
    ComponentId, DistributorError, DistributorSource, PackId, Version,
};

/// Filesystem layout strategies for the dev distributor source.
#[derive(Clone, Debug)]
pub enum DevLayout {
    Flat,
    ByIdAndVersion,
}

/// Configuration for [`DevDistributorSource`].
#[derive(Clone, Debug)]
pub struct DevConfig {
    pub root_dir: PathBuf,
    pub packs_dir: String,
    pub components_dir: String,
    pub layout: DevLayout,
}

impl Default for DevConfig {
    fn default() -> Self {
        DevConfig {
            root_dir: PathBuf::from(".greentic/dev"),
            packs_dir: "packs".into(),
            components_dir: "components".into(),
            layout: DevLayout::Flat,
        }
    }
}

/// Serves packs/components directly from a local directory tree.
pub struct DevDistributorSource {
    cfg: DevConfig,
}

impl DevDistributorSource {
    pub fn new(cfg: DevConfig) -> Self {
        Self { cfg }
    }

    fn pack_path(&self, pack_id: &PackId, version: &Version) -> PathBuf {
        match self.cfg.layout {
            DevLayout::Flat => self
                .root()
                .join(&self.cfg.packs_dir)
                .join(format!("{pack_id}-{version}.gtpack")),
            DevLayout::ByIdAndVersion => self
                .root()
                .join(&self.cfg.packs_dir)
                .join(pack_id.as_str())
                .join(version.to_string())
                .join("pack.gtpack"),
        }
    }

    fn component_path(&self, component_id: &ComponentId, version: &Version) -> PathBuf {
        match self.cfg.layout {
            DevLayout::Flat => self
                .root()
                .join(&self.cfg.components_dir)
                .join(format!("{component_id}-{version}.wasm")),
            DevLayout::ByIdAndVersion => self
                .root()
                .join(&self.cfg.components_dir)
                .join(component_id.as_str())
                .join(version.to_string())
                .join("component.wasm"),
        }
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, DistributorError> {
        match fs::read(path) {
            Ok(bytes) => Ok(bytes),
            Err(err) if err.kind() == ErrorKind::NotFound => Err(DistributorError::NotFound),
            Err(err) => Err(DistributorError::Io(err)),
        }
    }

    fn root(&self) -> &Path {
        &self.cfg.root_dir
    }
}

impl DistributorSource for DevDistributorSource {
    fn fetch_pack(&self, pack_id: &PackId, version: &Version) -> Result<Vec<u8>, DistributorError> {
        let path = self.pack_path(pack_id, version);
        self.read_file(&path)
    }

    fn fetch_component(
        &self,
        component_id: &ComponentId,
        version: &Version,
    ) -> Result<Vec<u8>, DistributorError> {
        let path = self.component_path(component_id, version);
        self.read_file(&path)
    }
}
