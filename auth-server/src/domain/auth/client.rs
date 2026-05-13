use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AppClient {
    pub client_id: String,
    pub client_secret: String,
    pub token_audience: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone)]
pub struct AppClients(Vec<AppClient>);

#[derive(Debug, Error)]
pub enum AppClientError {
    #[error("app clients must not be empty")]
    Empty,
    #[error("client_id must not be empty")]
    EmptyClientId,
    #[error("client_secret must not be empty")]
    EmptyClientSecret,
    #[error("token_audience must not be empty")]
    EmptyTokenAudience,
    #[error("redirect_uri must not be empty")]
    EmptyRedirectUri,
    #[error("duplicate client_id '{0}'")]
    DuplicateClientId(String),
    #[error("unknown client_id '{0}'")]
    UnknownClientId(String),
    #[error("invalid app client config: {0}")]
    InvalidConfig(String),
}

pub fn parse(raw: &str) -> Result<AppClients, AppClientError> {
    let clients: Vec<AppClient> =
        serde_json::from_str(raw).map_err(|e| AppClientError::InvalidConfig(e.to_string()))?;
    AppClients::new(clients)
}

impl AppClients {
    pub fn new(clients: Vec<AppClient>) -> Result<Self, AppClientError> {
        if clients.is_empty() {
            return Err(AppClientError::Empty);
        }

        let mut seen = std::collections::HashSet::new();
        for client in &clients {
            if client.client_id.trim().is_empty() {
                return Err(AppClientError::EmptyClientId);
            }
            if client.client_secret.trim().is_empty() {
                return Err(AppClientError::EmptyClientSecret);
            }
            if client.token_audience.trim().is_empty() {
                return Err(AppClientError::EmptyTokenAudience);
            }
            if client.redirect_uri.trim().is_empty() {
                return Err(AppClientError::EmptyRedirectUri);
            }
            if !seen.insert(client.client_id.clone()) {
                return Err(AppClientError::DuplicateClientId(client.client_id.clone()));
            }
        }

        Ok(Self(clients))
    }

    pub fn get(&self, client_id: &str) -> Result<&AppClient, AppClientError> {
        self.0
            .iter()
            .find(|client| client.client_id == client_id)
            .ok_or_else(|| AppClientError::UnknownClientId(client_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clients() {
        let clients = parse(
            r#"[{"client_id":"web","client_secret":"secret","token_audience":"api","redirect_uri":"https://app.example/cb"}]"#,
        )
        .unwrap();

        assert_eq!(clients.get("web").unwrap().token_audience, "api");
    }

    #[test]
    fn rejects_unknown_client() {
        let clients = parse(
            r#"[{"client_id":"web","client_secret":"secret","token_audience":"api","redirect_uri":"https://app.example/cb"}]"#,
        )
        .unwrap();

        assert!(matches!(
            clients.get("missing"),
            Err(AppClientError::UnknownClientId(_))
        ));
    }
}
