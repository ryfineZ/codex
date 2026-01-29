use codex_app_server_protocol::AuthMode;
use codex_backend_client::Client as BackendClient;
use codex_core::AuthManager;
use codex_core::config_loader::CloudRequirementsLoader;
use codex_core::config_loader::ConfigRequirementsToml;
use codex_protocol::account::PlanType;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::time::timeout;

const CLOUD_REQUIREMENTS_TIMEOUT: Duration = Duration::from_secs(5);

pub fn cloud_requirements_loader(
    auth_manager: Arc<AuthManager>,
    chatgpt_base_url: String,
) -> CloudRequirementsLoader {
    let task = tokio::spawn(async move {
        fetch_cloud_requirements_with_timeout(auth_manager, chatgpt_base_url).await
    });
    CloudRequirementsLoader::new(async move {
        match task.await {
            Ok(requirements) => requirements,
            Err(err) => {
                tracing::warn!(error = %err, "Cloud requirements task failed");
                None
            }
        }
    })
}

async fn fetch_cloud_requirements_with_timeout(
    auth_manager: Arc<AuthManager>,
    chatgpt_base_url: String,
) -> Option<ConfigRequirementsToml> {
    let started_at = Instant::now();
    let result = match timeout(
        CLOUD_REQUIREMENTS_TIMEOUT,
        fetch_cloud_requirements(auth_manager, chatgpt_base_url),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Timed out waiting for cloud requirements; continuing without them");
            return None;
        }
    };

    match result.as_ref() {
        Some(requirements) => {
            tracing::info!(
                elapsed_ms = started_at.elapsed().as_millis(),
                requirements = ?requirements,
                "Cloud requirements load completed"
            );
        }
        None => {
            tracing::info!(
                elapsed_ms = started_at.elapsed().as_millis(),
                "Cloud requirements load completed (none)"
            );
        }
    }

    result
}

async fn fetch_cloud_requirements(
    auth_manager: Arc<AuthManager>,
    chatgpt_base_url: String,
) -> Option<ConfigRequirementsToml> {
    let auth = auth_manager.auth().await?;
    if auth.mode != AuthMode::ChatGPT {
        return None;
    }
    if auth.account_plan_type() != Some(PlanType::Enterprise) {
        return None;
    }

    let client = match BackendClient::from_auth(chatgpt_base_url, &auth) {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "Failed to construct backend client for cloud requirements"
            );
            return None;
        }
    };

    let response = match client.get_config_requirements_file().await {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!(error = %err, "Failed to fetch cloud requirements");
            return None;
        }
    };

    let Some(contents) = response.contents else {
        tracing::warn!("Cloud requirements response missing contents");
        return None;
    };

    if contents.trim().is_empty() {
        return None;
    }

    let requirements: ConfigRequirementsToml = match toml::from_str(&contents) {
        Ok(requirements) => requirements,
        Err(err) => {
            tracing::warn!(error = %err, "Failed to parse cloud requirements");
            return None;
        }
    };

    if requirements.is_empty() {
        None
    } else {
        Some(requirements)
    }
}
