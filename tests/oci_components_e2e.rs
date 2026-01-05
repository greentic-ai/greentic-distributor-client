#![cfg(feature = "oci-components")]

use greentic_distributor_client::oci_components::{
    ComponentResolveOptions, ComponentsExtension, ComponentsMode, OciComponentResolver,
};

/// Optional GHCR E2E: set `OCI_E2E=1` (and optionally `OCI_E2E_REF`) to run.
#[tokio::test]
async fn fetches_public_component_from_ghcr() {
    if std::env::var("OCI_E2E").as_deref() != Ok("1") {
        eprintln!("skipping public GHCR E2E (set OCI_E2E=1 to enable)");
        return;
    }

    let reference = std::env::var("OCI_E2E_REF")
        .unwrap_or_else(|_| "ghcr.io/greentic-ai/components/templates:latest".into());
    let temp = tempfile::tempdir().expect("tempdir");
    let resolver: OciComponentResolver<
        greentic_distributor_client::oci_components::DefaultRegistryClient,
    > = OciComponentResolver::new(ComponentResolveOptions {
        allow_tags: true, // public tag allowed for E2E
        offline: false,
        cache_dir: temp.path().into(),
        ..ComponentResolveOptions::default()
    });

    let ext = ComponentsExtension {
        refs: vec![reference.clone()],
        mode: ComponentsMode::Eager,
    };
    let results = resolver
        .resolve_refs(&ext)
        .await
        .unwrap_or_else(|e| panic!("failed to pull {reference}: {e:?} (requires network to GHCR)"));
    let component = &results[0];
    assert!(component.path.exists(), "cached path missing");
    assert!(
        component.fetched_from_network,
        "expected network fetch on first E2E pull"
    );
    assert!(
        component.manifest_digest.is_some(),
        "manifest digest should be recorded for future verification"
    );
}
