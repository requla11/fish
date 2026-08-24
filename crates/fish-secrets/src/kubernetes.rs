use crate::manager::SecretManager;

pub struct KubernetesSecretManager {
    namespace: String,
}

impl KubernetesSecretManager {
    pub fn new(namespace: String) -> Self {
        Self { namespace }
    }
}

#[async_trait::async_trait]
impl SecretManager for KubernetesSecretManager {
    async fn get_secret(&self, key: &str) -> Result<String, anyhow::Error> {
        Err(anyhow::anyhow!(
            "Kubernetes secret integration is not implemented yet (requested `{key}` in namespace `{}`)",
            self.namespace
        ))
    }

    async fn inject_secrets(&self, command: &str) -> Result<String, anyhow::Error> {
        Err(anyhow::anyhow!(
            "Kubernetes secret injection is not implemented yet (command `{command}`)"
        ))
    }
}
