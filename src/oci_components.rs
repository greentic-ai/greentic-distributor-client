use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use oci_distribution::Reference;
use oci_distribution::client::{Client, ClientConfig, ClientProtocol, ImageData};
use oci_distribution::errors::OciDistributionError;
use oci_distribution::manifest::{
    IMAGE_MANIFEST_LIST_MEDIA_TYPE, IMAGE_MANIFEST_MEDIA_TYPE, OCI_IMAGE_INDEX_MEDIA_TYPE,
    OCI_IMAGE_MEDIA_TYPE,
};
use oci_distribution::secrets::RegistryAuth;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const OCI_ARTIFACT_MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.artifact.manifest.v1+json";
const DOCKER_MANIFEST_MEDIA_TYPE: &str = "application/vnd.docker.distribution.manifest.v2+json";
const DOCKER_MANIFEST_LIST_MEDIA_TYPE: &str =
    "application/vnd.docker.distribution.manifest.list.v2+json";

/// Accepted manifest media types when pulling components.
static DEFAULT_ACCEPTED_MANIFEST_TYPES: &[&str] = &[
    OCI_ARTIFACT_MANIFEST_MEDIA_TYPE,
    OCI_IMAGE_MEDIA_TYPE,
    OCI_IMAGE_INDEX_MEDIA_TYPE,
    IMAGE_MANIFEST_MEDIA_TYPE,
    IMAGE_MANIFEST_LIST_MEDIA_TYPE,
    DOCKER_MANIFEST_MEDIA_TYPE,
    DOCKER_MANIFEST_LIST_MEDIA_TYPE,
];

/// Preferred component layer media types.
static DEFAULT_LAYER_MEDIA_TYPES: &[&str] = &[
    "application/vnd.wasm.component.v1+wasm",
    "application/vnd.module.wasm.content.layer.v1+wasm",
    "application/vnd.greentic.component.manifest+json",
    "application/wasm",
    "application/octet-stream",
];

/// Greentic pack extension for components.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ComponentsExtension {
    pub refs: Vec<String>,
    #[serde(default)]
    pub mode: ComponentsMode,
}

/// Pull mode for components.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentsMode {
    #[default]
    Eager,
    Lazy,
}

/// Configuration for resolving OCI component references.
#[derive(Clone, Debug)]
pub struct ComponentResolveOptions {
    pub allow_tags: bool,
    pub offline: bool,
    pub cache_dir: PathBuf,
    pub accepted_manifest_types: Vec<String>,
    pub preferred_layer_media_types: Vec<String>,
}

impl Default for ComponentResolveOptions {
    fn default() -> Self {
        Self {
            allow_tags: false,
            offline: false,
            cache_dir: default_cache_root(),
            accepted_manifest_types: DEFAULT_ACCEPTED_MANIFEST_TYPES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            preferred_layer_media_types: DEFAULT_LAYER_MEDIA_TYPES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

/// Result of resolving a single component reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedComponent {
    pub original_reference: String,
    pub resolved_digest: String,
    pub media_type: String,
    pub path: PathBuf,
    pub fetched_from_network: bool,
    pub manifest_digest: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    original_reference: String,
    resolved_digest: String,
    media_type: String,
    fetched_at_unix_seconds: u64,
    size_bytes: u64,
    #[serde(default)]
    manifest_digest: Option<String>,
}

/// Resolve OCI component references with caching and offline support.
pub struct OciComponentResolver<C: RegistryClient = DefaultRegistryClient> {
    client: C,
    opts: ComponentResolveOptions,
    cache: OciCache,
}

impl Default for OciComponentResolver<DefaultRegistryClient> {
    fn default() -> Self {
        Self::new(ComponentResolveOptions::default())
    }
}

impl<C: RegistryClient> OciComponentResolver<C> {
    pub fn new(opts: ComponentResolveOptions) -> Self {
        let cache = OciCache::new(opts.cache_dir.clone());
        Self {
            client: C::default_client(),
            opts,
            cache,
        }
    }

    pub fn with_client(client: C, opts: ComponentResolveOptions) -> Self {
        let cache = OciCache::new(opts.cache_dir.clone());
        Self {
            client,
            opts,
            cache,
        }
    }

    pub async fn resolve_refs(
        &self,
        extension: &ComponentsExtension,
    ) -> Result<Vec<ResolvedComponent>, OciComponentError> {
        let mut results = Vec::with_capacity(extension.refs.len());
        for reference in &extension.refs {
            results.push(self.resolve_single(reference).await?);
        }
        Ok(results)
    }

    async fn resolve_single(
        &self,
        reference: &str,
    ) -> Result<ResolvedComponent, OciComponentError> {
        let parsed =
            Reference::try_from(reference).map_err(|e| OciComponentError::InvalidReference {
                reference: reference.to_string(),
                reason: e.to_string(),
            })?;

        if parsed.digest().is_none() && !self.opts.allow_tags {
            return Err(OciComponentError::DigestRequired {
                reference: reference.to_string(),
            });
        }

        let expected_digest = parsed.digest().map(normalize_digest);
        if let Some(expected_digest) = expected_digest.as_ref() {
            if let Some(hit) = self.cache.try_hit(expected_digest, reference) {
                return Ok(hit);
            }
            if self.opts.offline {
                return Err(OciComponentError::OfflineMissing {
                    reference: reference.to_string(),
                    digest: expected_digest.clone(),
                });
            }
        } else if self.opts.offline {
            return Err(OciComponentError::OfflineTaggedReference {
                reference: reference.to_string(),
            });
        }

        let accepted_layer_types = self
            .opts
            .preferred_layer_media_types
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        let image = self
            .client
            .pull(&parsed, &accepted_layer_types)
            .await
            .map_err(|source| OciComponentError::PullFailed {
                reference: reference.to_string(),
                source,
            })?;

        let chosen_layer = select_layer(
            &image.layers,
            &self.opts.preferred_layer_media_types,
            reference,
        )?;
        let resolved_digest = image
            .digest
            .clone()
            .or_else(|| chosen_layer.digest.clone())
            .unwrap_or_else(|| compute_digest(&chosen_layer.data));
        let manifest_digest = image.digest.clone();

        if let Some(expected) = expected_digest.as_ref()
            && expected != &resolved_digest
        {
            return Err(OciComponentError::DigestMismatch {
                reference: reference.to_string(),
                expected: expected.clone(),
                actual: resolved_digest.clone(),
            });
        }

        let path = self.cache.write(
            &resolved_digest,
            &chosen_layer.media_type,
            &chosen_layer.data,
            reference,
            manifest_digest.clone(),
        )?;

        Ok(ResolvedComponent {
            original_reference: reference.to_string(),
            resolved_digest,
            media_type: chosen_layer.media_type.clone(),
            path,
            fetched_from_network: true,
            manifest_digest,
        })
    }
}

fn select_layer<'a>(
    layers: &'a [PulledLayer],
    preferred_types: &[String],
    reference: &str,
) -> Result<&'a PulledLayer, OciComponentError> {
    if layers.is_empty() {
        return Err(OciComponentError::MissingLayers {
            reference: reference.to_string(),
        });
    }
    for ty in preferred_types {
        if let Some(layer) = layers.iter().find(|l| &l.media_type == ty) {
            return Ok(layer);
        }
    }
    Ok(&layers[0])
}

fn compute_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn normalize_digest(digest: &str) -> String {
    if digest.starts_with("sha256:") {
        digest.to_string()
    } else {
        format!("sha256:{digest}")
    }
}

pub(crate) fn default_cache_root() -> PathBuf {
    if let Ok(root) = std::env::var("GREENTIC_DIST_CACHE_DIR") {
        return PathBuf::from(root);
    }
    if let Some(cache) = dirs_next::cache_dir() {
        return cache.join("greentic").join("components");
    }
    if let Ok(root) = std::env::var("GREENTIC_HOME") {
        return PathBuf::from(root).join("cache").join("components");
    }
    PathBuf::from(".greentic").join("cache").join("components")
}

#[derive(Clone, Debug)]
struct OciCache {
    root: PathBuf,
}

impl OciCache {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn write(
        &self,
        digest: &str,
        media_type: &str,
        data: &[u8],
        reference: &str,
        manifest_digest: Option<String>,
    ) -> Result<PathBuf, OciComponentError> {
        let dir = self.artifact_dir(digest);
        fs::create_dir_all(&dir).map_err(|source| OciComponentError::Io {
            reference: reference.to_string(),
            source,
        })?;

        let artifact_path = dir.join("component.wasm");
        fs::write(&artifact_path, data).map_err(|source| OciComponentError::Io {
            reference: reference.to_string(),
            source,
        })?;

        let metadata = CacheMetadata {
            original_reference: reference.to_string(),
            resolved_digest: digest.to_string(),
            media_type: media_type.to_string(),
            fetched_at_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            size_bytes: data.len() as u64,
            manifest_digest,
        };
        let metadata_path = dir.join("metadata.json");
        let buf =
            serde_json::to_vec_pretty(&metadata).map_err(|source| OciComponentError::Serde {
                reference: reference.to_string(),
                source,
            })?;
        fs::write(&metadata_path, buf).map_err(|source| OciComponentError::Io {
            reference: reference.to_string(),
            source,
        })?;

        Ok(artifact_path)
    }

    fn try_hit(&self, digest: &str, reference: &str) -> Option<ResolvedComponent> {
        let path = self.artifact_path(digest);
        if !path.exists() {
            return None;
        }
        let metadata = self.read_metadata(digest).ok();
        let media_type = metadata
            .as_ref()
            .map(|m| m.media_type.clone())
            .unwrap_or_else(|| "application/octet-stream".to_string());
        Some(ResolvedComponent {
            original_reference: reference.to_string(),
            resolved_digest: digest.to_string(),
            media_type,
            path,
            fetched_from_network: false,
            manifest_digest: metadata.and_then(|m| m.manifest_digest),
        })
    }

    fn read_metadata(&self, digest: &str) -> anyhow::Result<CacheMetadata> {
        let metadata_path = self.metadata_path(digest);
        let bytes = fs::read(metadata_path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn artifact_dir(&self, digest: &str) -> PathBuf {
        self.root.join(trim_digest_prefix(digest))
    }

    fn artifact_path(&self, digest: &str) -> PathBuf {
        self.artifact_dir(digest).join("component.wasm")
    }

    fn metadata_path(&self, digest: &str) -> PathBuf {
        self.artifact_dir(digest).join("metadata.json")
    }
}

fn trim_digest_prefix(digest: &str) -> &str {
    digest
        .strip_prefix("sha256:")
        .unwrap_or_else(|| digest.trim_start_matches('@'))
}

#[derive(Clone, Debug)]
pub struct PulledImage {
    pub digest: Option<String>,
    pub layers: Vec<PulledLayer>,
}

#[derive(Clone, Debug)]
pub struct PulledLayer {
    pub media_type: String,
    pub data: Vec<u8>,
    pub digest: Option<String>,
}

#[async_trait]
pub trait RegistryClient: Send + Sync {
    fn default_client() -> Self
    where
        Self: Sized;

    async fn pull(
        &self,
        reference: &Reference,
        accepted_manifest_types: &[&str],
    ) -> Result<PulledImage, OciDistributionError>;
}

/// Registry client backed by `oci-distribution` with HTTPS enforced and anonymous pulls.
#[derive(Clone)]
pub struct DefaultRegistryClient {
    inner: Client,
}

impl Default for DefaultRegistryClient {
    fn default() -> Self {
        Self::default_client()
    }
}

#[async_trait]
impl RegistryClient for DefaultRegistryClient {
    fn default_client() -> Self {
        let config = ClientConfig {
            protocol: ClientProtocol::Https,
            ..Default::default()
        };
        Self {
            inner: Client::new(config),
        }
    }

    async fn pull(
        &self,
        reference: &Reference,
        accepted_manifest_types: &[&str],
    ) -> Result<PulledImage, OciDistributionError> {
        let image = self
            .inner
            .pull(
                reference,
                &RegistryAuth::Anonymous,
                accepted_manifest_types.to_vec(),
            )
            .await?;
        Ok(convert_image(image))
    }
}

fn convert_image(image: ImageData) -> PulledImage {
    let layers = image
        .layers
        .into_iter()
        .map(|layer| {
            let digest = format!("sha256:{}", layer.sha256_digest());
            PulledLayer {
                media_type: layer.media_type,
                data: layer.data,
                digest: Some(digest),
            }
        })
        .collect();
    PulledImage {
        digest: image.digest,
        layers,
    }
}

#[derive(Debug, Error)]
pub enum OciComponentError {
    #[error("invalid OCI reference `{reference}`: {reason}")]
    InvalidReference { reference: String, reason: String },
    #[error("digest pin required for `{reference}` (rerun with --allow-tags to permit tag refs)")]
    DigestRequired { reference: String },
    #[error("offline mode prohibits tagged reference `{reference}`; pin by digest first")]
    OfflineTaggedReference { reference: String },
    #[error("offline mode could not find cached component for `{reference}` (digest `{digest}`)")]
    OfflineMissing { reference: String, digest: String },
    #[error("no layers returned for `{reference}`")]
    MissingLayers { reference: String },
    #[error("component layer missing for `{reference}`; tried media types {media_types}")]
    MissingComponent {
        reference: String,
        media_types: String,
    },
    #[error("digest mismatch for `{reference}`: expected {expected}, got {actual}")]
    DigestMismatch {
        reference: String,
        expected: String,
        actual: String,
    },
    #[error("failed to pull `{reference}`: {source}")]
    PullFailed {
        reference: String,
        #[source]
        source: oci_distribution::errors::OciDistributionError,
    },
    #[error("io error while caching `{reference}`: {source}")]
    Io {
        reference: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize cache metadata for `{reference}`: {source}")]
    Serde {
        reference: String,
        #[source]
        source: serde_json::Error,
    },
}
