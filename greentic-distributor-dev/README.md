# greentic-distributor-dev

Dev-only distributor source that serves packs and components directly from the local filesystem. Useful for running flows locally without publishing packs/components to a remote store.

## Layouts
- `Flat` (default)
  - Packs: `{root}/{packs_dir}/{pack_id}-{version}.gtpack`
  - Components: `{root}/{components_dir}/{component_id}-{version}.wasm`
- `ByIdAndVersion`
  - Packs: `{root}/{packs_dir}/{pack_id}/{version}/pack.gtpack`
  - Components: `{root}/{components_dir}/{component_id}/{version}/component.wasm`

Default config points at `.greentic/dev` with `packs` and `components` subdirectories.

## Usage
```rust
use greentic_distributor_client::{ChainedDistributorSource, DistributorSource, PackId, Version};
use greentic_distributor_dev::{DevConfig, DevDistributorSource};

let dev_source = DevDistributorSource::new(DevConfig::default());
let sources = ChainedDistributorSource::new(vec![Box::new(dev_source)]);

let pack_id = PackId::try_from("dev.local.hello-flow")?;
let version = Version::parse("0.1.0")?;
let pack_bytes = sources.fetch_pack(&pack_id, &version)?;
println!("loaded {} bytes", pack_bytes.len());
```

Future greentic-dev integration can build packs/components into `.greentic/dev/{packs,components}` and resolve them via this source without any remote distributor.
