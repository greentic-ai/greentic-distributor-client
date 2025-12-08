# Repository Overview

## 1. High-Level Purpose
- Workspace providing a distributor client plus a dev-only filesystem-backed distributor source.
- `greentic-distributor-client`: async client trait with HTTP and WIT implementations; uses `greentic-types` DTOs and `greentic-interfaces-guest` bindings.
- `greentic-distributor-dev`: implements `DistributorSource` to serve packs/components from a local directory (flat or nested layouts) for local/dev flows.

## 2. Main Components and Functionality
- **Path:** `src/lib.rs`
  - **Role:** Library entrypoint exporting the `DistributorClient` trait plus HTTP/WIT client implementations, shared types/config/errors, and source abstraction.
  - **Key functionality:** Async trait with `resolve_component`, `get_pack_status`, `warm_pack`; re-exports config/DTOs and `DistributorSource` composition helpers.
- **Path:** `src/types.rs`
  - **Role:** Re-exports distributor DTOs and IDs from `greentic-types` (TenantCtx, DistributorEnvironmentId, ComponentDigest/status, ArtifactLocation, SignatureSummary, CacheInfo, resolve request/response, EnvId/TenantId/ComponentId/PackId).
  - **Key functionality:** Leverages upstream helpers (e.g., `is_sha256_like` on ComponentDigest) and re-exports `semver::Version`.
- **Path:** `src/config.rs`
  - **Role:** Client configuration (base URL optional for HTTP, tenant/environment IDs, optional bearer token, extra headers, timeout).
- **Path:** `src/error.rs`
  - **Role:** `DistributorError` enum covering HTTP, WIT, IO/config/other errors, invalid response, not-found/permission errors, serde issues.
- **Path:** `src/source.rs`
  - **Role:** `DistributorSource` trait for pack/component fetching plus `ChainedDistributorSource` for priority lookup; includes in-memory tests.
- **Path:** `src/http.rs`
  - **Role:** `HttpDistributorClient` implementing the trait over JSON endpoints (`/distributor-api/resolve-component`, `/pack-status`, `/warm-pack`); injects auth/headers and maps HTTP statuses.
- **Path:** `src/wit_client.rs`
  - **Role:** `WitDistributorClient` plus `DistributorApiBindings` trait to wrap actual WIT guest bindings; provides `GeneratedDistributorApiBindings` that calls distributor-api imports on WASM targets (errors on non-WASM) and handles DTO↔WIT conversions using `greentic-interfaces-guest::distributor_api` types and JSON parsing.
- **Path:** `tests/http_client.rs`
  - **Role:** HTTP client tests using `httpmock` for success, pack-status JSON, auth header propagation, 404 error mapping, server-error mapping, and digest validation.
- **Path:** `tests/wit_client.rs`
  - **Role:** WIT translation tests against a dummy binding verifying DTO↔WIT conversions and JSON parsing/warm-pack call-through.
- **Path:** `greentic-distributor-dev/src/lib.rs`
  - **Role:** `DevDistributorSource` implementation reading packs/components from local disk using configurable `DevConfig` and `DevLayout` (Flat or ByIdAndVersion).
- **Path:** `greentic-distributor-dev/tests/dev_source.rs`
  - **Role:** Integration tests covering flat/nested layouts, happy paths, and not-found.
- **Path:** `README.md` / `LICENSE`
  - **Role:** Crate metadata for publication (MIT license, description/usage overview, local dev distributor usage example).
- **Path:** `.github/workflows/ci.yml`
  - **Role:** CI workflow running fmt, clippy, tests, and a packaging check on pushes/PRs.
- **Path:** `.github/workflows/publish.yml`
  - **Role:** Publish workflow for crates.io on tags or manual dispatch using `CARGO_REGISTRY_TOKEN`; packages/publishes both client and dev crates.
- **Path:** `ci/local_check.sh`
  - **Role:** Local helper script running fmt, clippy, and tests.
- **Path:** `README.md` / `LICENSE`
  - **Role:** Crate metadata for publication (MIT license, description/usage overview).

## 3. Work In Progress, TODOs, and Stubs
- No explicit TODO markers. WIT integration uses a pluggable `DistributorApiBindings` trait; `GeneratedDistributorApiBindings` works on WASM targets and errors on non-WASM (where HTTP is expected). Dev distributor is ready for greentic-dev wiring.

## 4. Broken, Failing, or Conflicting Areas
- None observed. `cargo test` now passes (HTTP client + WIT translation tests).

## 5. Notes for Future Work
- Confirm HTTP JSON field naming against the canonical distributor API and adjust serializers as needed.
- Keep packaging checks in CI; publish workflow already runs `cargo package` for both crates.
