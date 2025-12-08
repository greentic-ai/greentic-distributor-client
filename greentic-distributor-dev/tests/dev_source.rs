use std::fs;

use greentic_distributor_client::{
    ComponentId, DistributorError, DistributorSource, PackId, Version,
};
use greentic_distributor_dev::{DevConfig, DevDistributorSource, DevLayout};
use tempfile::tempdir;

#[test]
fn fetches_from_flat_layout() {
    let root = tempdir().unwrap();
    let packs_dir = root.path().join("packs");
    let components_dir = root.path().join("components");
    fs::create_dir_all(&packs_dir).unwrap();
    fs::create_dir_all(&components_dir).unwrap();

    let pack_id = PackId::try_from("dev.local.hello-flow").unwrap();
    let component_id = ComponentId::try_from("dev.greentic.echo").unwrap();
    let version = Version::parse("0.1.0").unwrap();

    fs::write(
        packs_dir.join("dev.local.hello-flow-0.1.0.gtpack"),
        b"pack-bytes",
    )
    .unwrap();
    fs::write(
        components_dir.join("dev.greentic.echo-0.1.0.wasm"),
        b"component-bytes",
    )
    .unwrap();

    let cfg = DevConfig {
        root_dir: root.path().to_path_buf(),
        ..Default::default()
    };
    let source = DevDistributorSource::new(cfg);

    let pack_bytes = source.fetch_pack(&pack_id, &version).unwrap();
    assert_eq!(pack_bytes, b"pack-bytes");

    let component_bytes = source.fetch_component(&component_id, &version).unwrap();
    assert_eq!(component_bytes, b"component-bytes");
}

#[test]
fn fetches_from_nested_layout() {
    let root = tempdir().unwrap();
    let pack_id = PackId::try_from("dev.local.hello-flow").unwrap();
    let component_id = ComponentId::try_from("dev.greentic.echo").unwrap();
    let version = Version::parse("0.1.0").unwrap();

    let pack_path = root
        .path()
        .join("packs")
        .join(pack_id.as_str())
        .join(version.to_string());
    let component_path = root
        .path()
        .join("components")
        .join(component_id.as_str())
        .join(version.to_string());
    fs::create_dir_all(&pack_path).unwrap();
    fs::create_dir_all(&component_path).unwrap();
    fs::write(pack_path.join("pack.gtpack"), b"pack-nested").unwrap();
    fs::write(component_path.join("component.wasm"), b"component-nested").unwrap();

    let cfg = DevConfig {
        root_dir: root.path().to_path_buf(),
        layout: DevLayout::ByIdAndVersion,
        ..Default::default()
    };
    let source = DevDistributorSource::new(cfg);

    let pack_bytes = source.fetch_pack(&pack_id, &version).unwrap();
    assert_eq!(pack_bytes, b"pack-nested");

    let component_bytes = source.fetch_component(&component_id, &version).unwrap();
    assert_eq!(component_bytes, b"component-nested");
}

#[test]
fn returns_not_found_when_missing() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("packs")).unwrap();
    fs::create_dir_all(root.path().join("components")).unwrap();

    let cfg = DevConfig {
        root_dir: root.path().to_path_buf(),
        ..Default::default()
    };
    let source = DevDistributorSource::new(cfg);
    let pack_id = PackId::try_from("dev.missing").unwrap();
    let version = Version::parse("0.1.0").unwrap();

    let err = source.fetch_pack(&pack_id, &version).unwrap_err();
    assert!(matches!(err, DistributorError::NotFound));
}
