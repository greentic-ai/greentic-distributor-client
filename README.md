# greentic-distributor-client

WIT-based client for the `greentic:distributor-api@1.0.0` world. Provides:
- `DistributorClient` async trait for resolving components, querying pack status, and warming packs.
- `WitDistributorClient` adapter that translates DTOs to `greentic-interfaces-guest` distributor-api bindings; use `GeneratedDistributorApiBindings` on WASM targets to call the distributor imports.
- Optional HTTP runtime client behind the `http-runtime` feature for JSON endpoints that mirror the runtime API.

Uses DTOs from `greentic-types`.

## Usage

```rust
use greentic_distributor_client::{
    DistributorApiBindings, DistributorClient, DistributorEnvironmentId, EnvId,
    GeneratedDistributorApiBindings, ResolveComponentRequest, TenantCtx, TenantId,
    WitDistributorClient,
};
use serde_json::json;

let bindings = GeneratedDistributorApiBindings::default();
let client = WitDistributorClient::new(bindings);
let resp = client.resolve_component(ResolveComponentRequest {
    tenant: TenantCtx::new(
        EnvId::try_from("prod").unwrap(),
        TenantId::try_from("tenant-a").unwrap(),
    ),
    environment_id: DistributorEnvironmentId::from("env-1"),
    pack_id: "pack-123".into(),
    component_id: "component-x".into(),
    version: "1.0.0".into(),
    extra: json!({}),
}).await?;
println!("artifact: {:?}", resp.artifact);
println!("secret requirements: {:?}", resp.secret_requirements);
```

`GeneratedDistributorApiBindings` calls the distributor imports on WASM targets. On non-WASM targets it returns an error; consumers can provide their own bindings implementation for testing.

### HTTP runtime client (feature `http-runtime`)
Enable the feature and construct `HttpDistributorClient`:

```toml
[dependencies]
greentic-distributor-client = { version = "0.4", features = ["http-runtime"] }
```

```rust
use greentic_distributor_client::{
    DistributorClient, DistributorClientConfig, DistributorEnvironmentId, EnvId, HttpDistributorClient,
    ResolveComponentRequest, TenantCtx, TenantId,
};
use serde_json::json;

let config = DistributorClientConfig {
    base_url: Some("https://distributor.example.com".into()),
    environment_id: DistributorEnvironmentId::from("env-1"),
    tenant: TenantCtx::new(EnvId::try_from("prod").unwrap(), TenantId::try_from("tenant-a").unwrap()),
    auth_token: Some("token123".into()),
    extra_headers: None,
    request_timeout: None,
};
let client = HttpDistributorClient::new(config)?;
let resp = client.resolve_component(ResolveComponentRequest {
    tenant: TenantCtx::new(
        EnvId::try_from("prod").unwrap(),
        TenantId::try_from("tenant-a").unwrap(),
    ),
    environment_id: DistributorEnvironmentId::from("env-1"),
    pack_id: "pack-123".into(),
    component_id: "component-x".into(),
    version: "1.0.0".into(),
    extra: json!({}),
}).await?;
println!("artifact: {:?}", resp.artifact);
println!("secret requirements: {:?}", resp.secret_requirements);
```

Fetch typed pack status (includes secret requirements):

```rust
let status = client
    .get_pack_status_v2(
        &TenantCtx::new(EnvId::try_from("prod")?, TenantId::try_from("tenant-a")?),
        &DistributorEnvironmentId::from("env-1"),
        "pack-123",
    )
    .await?;
println!("status: {}, secrets: {:?}", status.status, status.secret_requirements);
```

## Local dev distributor
Use the companion `greentic-distributor-dev` crate to serve packs/components from a local directory, useful for greentic-dev and conformance tests:

```rust
use greentic_distributor_client::{ChainedDistributorSource, DistributorSource, PackId, Version};
use greentic_distributor_dev::{DevConfig, DevDistributorSource};

let dev_source = DevDistributorSource::new(DevConfig::default());
let sources = ChainedDistributorSource::new(vec![Box::new(dev_source)]);

let pack_bytes = sources.fetch_pack(&PackId::try_from("dev.local.hello-flow")?, &Version::parse("0.1.0")?);
println!("Loaded {} bytes", pack_bytes.len());
```

## Repo maintenance
- Enable GitHub's "Allow auto-merge" setting for the repository.
- Configure branch protection with the required checks you want enforced before merges.
