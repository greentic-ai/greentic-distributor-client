#![cfg(feature = "dist-client")]

use greentic_distributor_client::dist::{DistClient, DistOptions};
use sha2::{Digest, Sha256};
use std::fs;
use tempfile::TempDir;

fn digest_for(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn options(dir: &TempDir) -> DistOptions {
    DistOptions {
        cache_dir: dir.path().to_path_buf(),
        allow_tags: true,
        offline: false,
    }
}

#[tokio::test]
async fn caches_file_path_and_computes_digest() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("component.wasm");
    fs::write(&file_path, b"hello-component").unwrap();
    let expected = digest_for(b"hello-component");

    let client = DistClient::new(options(&temp));
    let resolved = client
        .ensure_cached(file_path.to_str().unwrap())
        .await
        .unwrap();

    assert_eq!(resolved.digest, expected);
    let cached = resolved.cache_path.unwrap();
    assert!(cached.exists());
    assert_eq!(fs::read(cached).unwrap(), b"hello-component");
}

#[tokio::test]
async fn caches_http_download() {
    let server = match std::panic::catch_unwind(|| httpmock::MockServer::start()) {
        Ok(s) => s,
        Err(_) => {
            eprintln!(
                "skipping http download test: unable to bind mock server in this environment"
            );
            return;
        }
    };
    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/component.wasm");
        then.status(200).body("from-http");
    });

    let temp = tempfile::tempdir().unwrap();
    let client = DistClient::new(options(&temp));
    let url = format!("{}/component.wasm", server.base_url());
    let resolved = client.ensure_cached(&url).await.unwrap();

    let expected = digest_for(b"from-http");
    assert_eq!(resolved.digest, expected);
    let cached = resolved.cache_path.unwrap();
    assert_eq!(fs::read(cached).unwrap(), b"from-http");
    mock.assert_async().await;
}

#[tokio::test]
async fn pulls_lockfile_entries() {
    let temp = tempfile::tempdir().unwrap();
    let file1 = temp.path().join("one.wasm");
    let file2 = temp.path().join("two.wasm");
    fs::write(&file1, b"one").unwrap();
    fs::write(&file2, b"two").unwrap();

    let lock_path = temp.path().join("pack.lock");
    let lock_contents = serde_json::json!({
        "components": [
            file1.to_str().unwrap(),
            { "reference": file2.to_str().unwrap() }
        ]
    });
    fs::write(&lock_path, serde_json::to_vec(&lock_contents).unwrap()).unwrap();

    let client = DistClient::new(options(&temp));
    let resolved = client.pull_lock(&lock_path).await.unwrap();

    assert_eq!(resolved.len(), 2);
    for item in resolved {
        assert!(item.cache_path.unwrap().exists());
    }
}

#[tokio::test]
async fn respects_canonical_lockfile_with_schema_version() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("hello.wasm");
    fs::write(&file, b"hello").unwrap();
    let digest = digest_for(b"hello");
    let lock_path = temp.path().join("pack.lock.json");
    let lock_contents = serde_json::json!({
        "schema_version": 1,
        "components": [
            {
                "name": "hello",
                "ref": file.to_str().unwrap(),
                "digest": digest
            }
        ]
    });
    fs::write(&lock_path, serde_json::to_vec(&lock_contents).unwrap()).unwrap();

    let client = DistClient::new(options(&temp));
    let resolved = client.pull_lock(&lock_path).await.unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].digest, digest);
    assert!(resolved[0].cache_path.as_ref().unwrap().exists());
}

#[tokio::test]
async fn offline_mode_blocks_http_fetch() {
    let temp = tempfile::tempdir().unwrap();
    let mut opts = options(&temp);
    opts.offline = true;
    let client = DistClient::new(opts);
    let err = client
        .resolve_ref("http://example.com/component.wasm")
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("offline"), "unexpected error: {msg}");
}
