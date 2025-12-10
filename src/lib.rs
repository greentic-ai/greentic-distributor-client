pub mod config;
pub mod error;
pub mod source;
pub mod types;

#[cfg(feature = "http-runtime")]
mod http;
mod wit_client;

pub use config::DistributorClientConfig;
pub use error::DistributorError;
#[cfg(feature = "http-runtime")]
pub use http::HttpDistributorClient;
pub use source::{ChainedDistributorSource, DistributorSource};
pub use types::*;
pub use wit_client::{
    DistributorApiBindings, GeneratedDistributorApiBindings, WitDistributorClient,
};

use async_trait::async_trait;

/// Trait implemented by clients that can communicate with a Distributor.
#[async_trait]
pub trait DistributorClient: Send + Sync {
    async fn resolve_component(
        &self,
        req: ResolveComponentRequest,
    ) -> Result<ResolveComponentResponse, DistributorError>;

    async fn get_pack_status(
        &self,
        tenant: &TenantCtx,
        env: &DistributorEnvironmentId,
        pack_id: &str,
    ) -> Result<serde_json::Value, DistributorError>;

    async fn warm_pack(
        &self,
        tenant: &TenantCtx,
        env: &DistributorEnvironmentId,
        pack_id: &str,
    ) -> Result<(), DistributorError>;
}
