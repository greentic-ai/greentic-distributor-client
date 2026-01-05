#![cfg(feature = "oci-components")]

use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use greentic_distributor_client::oci_components::{
    ComponentResolveOptions, ComponentsExtension, ComponentsMode, OciComponentError,
    OciComponentResolver, PulledImage, PulledLayer, RegistryClient, ResolvedComponent,
};
use oci_distribution::Reference;
use oci_distribution::errors::OciDistributionError;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[derive(Clone, Default)]
struct MockRegistryClient {
    pulls: Arc<AtomicUsize>,
    images: Arc<Mutex<HashMap<String, PulledImage>>>,
}

impl MockRegistryClient {
    fn with_image(reference: &str, image: PulledImage) -> Self {
        let client = Self::default();
        client
            .images
            .lock()
            .unwrap()
            .insert(reference.to_string(), image);
        client
    }

    fn pulls(&self) -> usize {
        self.pulls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl RegistryClient for MockRegistryClient {
    fn default_client() -> Self {
        Self::default()
    }

    async fn pull(
        &self,
        reference: &Reference,
        _accepted_manifest_types: &[&str],
    ) -> Result<PulledImage, OciDistributionError> {
        self.pulls.fetch_add(1, Ordering::SeqCst);
        let key = reference.whole();
        self.images
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .ok_or_else(|| OciDistributionError::GenericError(Some("not found".into())))
    }
}

fn options(temp: &TempDir) -> ComponentResolveOptions {
    ComponentResolveOptions {
        cache_dir: temp.path().to_path_buf(),
        ..ComponentResolveOptions::default()
    }
}

fn digest_for(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{:x}", hasher.finalize())
}

fn extension(refs: Vec<&str>) -> ComponentsExtension {
    ComponentsExtension {
        refs: refs.into_iter().map(|s| s.to_string()).collect(),
        mode: ComponentsMode::Eager,
    }
}

fn pulled_image(data: &[u8], media_type: &str, digest: &str) -> PulledImage {
    PulledImage {
        digest: Some(digest.to_string()),
        layers: vec![PulledLayer {
            media_type: media_type.to_string(),
            data: data.to_vec(),
            digest: Some(digest.to_string()),
        }],
    }
}

#[tokio::test]
async fn resolves_digest_pinned_and_caches() {
    let temp = tempfile::tempdir().unwrap();
    let data = b"wasm-bytes";
    let digest = digest_for(data);
    let reference = format!("ghcr.io/greentic/components@{digest}");

    let mock =
        MockRegistryClient::with_image(&reference, pulled_image(data, "application/wasm", &digest));
    let resolver = OciComponentResolver::with_client(mock.clone(), options(&temp));

    let results = resolver
        .resolve_refs(&extension(vec![&reference]))
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let ResolvedComponent {
        path,
        resolved_digest,
        fetched_from_network,
        manifest_digest,
        ..
    } = &results[0];
    assert_eq!(resolved_digest, &digest);
    assert!(path.exists());
    assert!(fetched_from_network);
    assert_eq!(manifest_digest.as_deref(), Some(digest.as_str()));
    assert_eq!(mock.pulls(), 1);

    // Second resolution should hit cache and avoid another pull.
    let results = resolver
        .resolve_refs(&extension(vec![&reference]))
        .await
        .unwrap();
    assert!(!results[0].fetched_from_network);
    assert_eq!(mock.pulls(), 1);
}

#[tokio::test]
async fn tag_refs_rejected_without_opt_in() {
    let temp = tempfile::tempdir().unwrap();
    let resolver = OciComponentResolver::with_client(MockRegistryClient::default(), options(&temp));
    let err = resolver
        .resolve_refs(&extension(vec!["ghcr.io/greentic/components:latest"]))
        .await
        .unwrap_err();
    assert!(matches!(err, OciComponentError::DigestRequired { .. }));
}

#[tokio::test]
async fn offline_mode_requires_cache() {
    let temp = tempfile::tempdir().unwrap();
    let data = b"component bytes";
    let digest = digest_for(data);
    let reference = format!("ghcr.io/greentic/components@{digest}");
    let mut opts = options(&temp);
    opts.offline = true;
    let resolver = OciComponentResolver::with_client(MockRegistryClient::default(), opts);

    let err = resolver
        .resolve_refs(&extension(vec![&reference]))
        .await
        .unwrap_err();
    assert!(matches!(err, OciComponentError::OfflineMissing { .. }));
}

#[tokio::test]
async fn allows_tag_refs_when_opted_in() {
    let temp = tempfile::tempdir().unwrap();
    let data = b"tagged component";
    let digest = digest_for(data);
    let reference = "ghcr.io/greentic/components:latest";

    let mut opts = options(&temp);
    opts.allow_tags = true;
    let mock =
        MockRegistryClient::with_image(reference, pulled_image(data, "application/wasm", &digest));
    let resolver = OciComponentResolver::with_client(mock.clone(), opts);

    let results = resolver
        .resolve_refs(&extension(vec![reference]))
        .await
        .unwrap();
    assert_eq!(results[0].resolved_digest, digest);
    assert!(results[0].path.exists());
    assert_eq!(mock.pulls(), 1);
}

#[tokio::test]
async fn invalid_reference_surfaces_error() {
    let temp = tempfile::tempdir().unwrap();
    let resolver = OciComponentResolver::with_client(MockRegistryClient::default(), options(&temp));
    let err = resolver
        .resolve_refs(&extension(vec!["not a ref"]))
        .await
        .unwrap_err();
    assert!(matches!(err, OciComponentError::InvalidReference { .. }));
}
