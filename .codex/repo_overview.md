# Repository Overview

## 1. High-Level Purpose
- Rust crate for a distributor client: uses `greentic-types` DTOs and `greentic-interfaces-guest` bindings (distributor-api feature) to expose an async client trait, HTTP implementation, and WIT adapter.
- Supports resolve/get-status/warm-pack flows with configurable auth headers/timeouts and serde-compatible models.

## 2. Main Components and Functionality
- **Path:** `src/lib.rs`
  - **Role:** Library entrypoint exporting the `DistributorClient` trait plus HTTP and WIT client implementations and shared types/config/errors.
  - **Key functionality:** Async trait with `resolve_component`, `get_pack_status`, `warm_pack`; re-exports config and DTOs for consumers.
- **Path:** `src/types.rs`
  - **Role:** Re-exports distributor DTOs and IDs from `greentic-types` (TenantCtx, DistributorEnvironmentId, ComponentDigest/status, ArtifactLocation, SignatureSummary, CacheInfo, resolve request/response, EnvId/TenantId/ComponentId/PackId).
  - **Key functionality:** Leverages upstream helpers (e.g., `is_sha256_like` on ComponentDigest).
- **Path:** `src/config.rs`
  - **Role:** Client configuration (base URL optional for HTTP, tenant/environment IDs, optional bearer token, extra headers, timeout).
- **Path:** `src/error.rs`
  - **Role:** `DistributorError` enum covering HTTP, WIT, invalid response, not-found/permission errors, and serde issues.
- **Path:** `src/http.rs`
  - **Role:** `HttpDistributorClient` implementing the trait over JSON endpoints (`/distributor-api/resolve-component`, `/pack-status`, `/warm-pack`); injects auth/headers and maps HTTP statuses.
- **Path:** `src/wit_client.rs`
  - **Role:** `WitDistributorClient` plus `DistributorApiBindings` trait to wrap actual WIT guest bindings; provides `GeneratedDistributorApiBindings` that calls distributor-api imports on WASM targets (errors on non-WASM) and handles DTO↔WIT conversions using `greentic-interfaces-guest::distributor_api` types and JSON parsing.
- **Path:** `tests/http_client.rs`
  - **Role:** HTTP client tests using `httpmock` for success, pack-status JSON, auth header propagation, 404 error mapping, server-error mapping, and digest validation.
- **Path:** `tests/wit_client.rs`
  - **Role:** WIT translation tests against a dummy binding verifying DTO↔WIT conversions and JSON parsing/warm-pack call-through.
- **Path:** `README.md` / `LICENSE`
  - **Role:** Crate metadata for publication (MIT license, description/usage overview).
- **Path:** `.github/workflows/ci.yml`
  - **Role:** CI workflow running fmt, clippy, tests, and a packaging check on pushes/PRs.
- **Path:** `.github/workflows/publish.yml`
  - **Role:** Publish workflow for crates.io on tags or manual dispatch using `CARGO_REGISTRY_TOKEN`, with a packaging pre-check.
- **Path:** `ci/local_check.sh`
  - **Role:** Local helper script running fmt, clippy, and tests.
- **Path:** `README.md` / `LICENSE`
  - **Role:** Crate metadata for publication (MIT license, description/usage overview).

## 3. Work In Progress, TODOs, and Stubs
- No explicit TODO markers. WIT integration uses a pluggable `DistributorApiBindings` trait; `GeneratedDistributorApiBindings` works on WASM targets and errors on non-WASM (where HTTP is expected).

## 4. Broken, Failing, or Conflicting Areas
- None observed. `cargo test` now passes (HTTP client + WIT translation tests).

## 5. Notes for Future Work
- Confirm HTTP JSON field naming against the canonical distributor API and adjust serializers as needed.
- Keep packaging checks in CI; publish workflow already runs `cargo package`.
