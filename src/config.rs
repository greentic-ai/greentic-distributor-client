use crate::{DistributorEnvironmentId, TenantCtx};
use std::{collections::HashMap, time::Duration};

/// Configuration for distributor clients.
///
/// NOTE: `base_url`, `auth_token`, and header fields are used when the
/// `http-runtime` feature is enabled. Without that feature, the WIT client is
/// the primary implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistributorClientConfig {
    pub base_url: Option<String>,
    pub environment_id: DistributorEnvironmentId,
    pub tenant: TenantCtx,
    pub auth_token: Option<String>,
    pub extra_headers: Option<HashMap<String, String>>,
    pub request_timeout: Option<Duration>,
}

impl DistributorClientConfig {
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
}
