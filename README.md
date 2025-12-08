# greentic-distributor-client

Client trait and HTTP/WIT implementations for the `greentic:distributor-api@1.0.0` world. Provides:
- `DistributorClient` async trait for resolving components, querying pack status, and warming packs.
- `HttpDistributorClient` for JSON/HTTP endpoints.
- `WitDistributorClient` adapter that translates DTOs to `greentic-interfaces-guest` distributor-api bindings; use `GeneratedDistributorApiBindings` on WASM targets to call the distributor imports.

Uses DTOs from `greentic-types` and supports optional auth headers and request timeouts.

## Usage

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
    auth_token: None,
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
```

For WIT, implement `DistributorApiBindings` using the generated `greentic_interfaces_guest::distributor_api` functions and pass it to `WitDistributorClient`.
`GeneratedDistributorApiBindings` is provided for WASM targets. On non-WASM targets it returns an error; prefer `HttpDistributorClient` there.

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
