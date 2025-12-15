# Migration status — secrets move into greentic-types
- What changed: Re-exported canonical secret types (`SecretRequirement`, `SecretKey`, `SecretScope`, `SecretFormat`) from `greentic-types` and bumped `greentic-interfaces(-guest/-wasmtime)` to 0.4.65 (distributor WIT now exposes `secret_requirements` and `get-pack-status-v2`).
- Current status: Completed in this repo — WIT-backed client now maps `secret_requirements` on resolve responses and exposes typed `get_pack_status_v2` carrying secret requirements.
- Next steps:
  - Downstream consumers (runner/deployer/dev tooling) should read the new fields and switch to `get_pack_status_v2` where possible.
  - Optional: bump `greentic-distributor-client` consumers and pin the new version.
