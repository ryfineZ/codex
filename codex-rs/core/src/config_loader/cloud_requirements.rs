use crate::config_loader::ConfigRequirementsToml;
use futures::future::BoxFuture;
use futures::future::FutureExt;
use futures::future::Shared;
use std::fmt;
use std::future::Future;

#[derive(Clone)]
pub struct CloudRequirementsLoader {
    fut: Shared<BoxFuture<'static, Option<ConfigRequirementsToml>>>,
}

impl CloudRequirementsLoader {
    pub fn new<F>(fut: F) -> Self
    where
        F: Future<Output = Option<ConfigRequirementsToml>> + Send + 'static,
    {
        Self {
            fut: fut.boxed().shared(),
        }
    }

    pub async fn get(&self) -> Option<ConfigRequirementsToml> {
        self.fut.clone().await
    }
}

impl fmt::Debug for CloudRequirementsLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudRequirementsLoader").finish()
    }
}
