use matrix_sdk::matrix_auth::{MatrixSession, MatrixSessionTokens};
use matrix_sdk::ruma::{OwnedDeviceId, OwnedUserId};
use matrix_sdk::Client;

use crate::config::{self, StoredSession};

pub async fn create_client(homeserver: &str) -> Result<Client, String> {
    let db_path = config::data_dir().join("matrix-store");

    Client::builder()
        .server_name_or_homeserver_url(homeserver)
        .sqlite_store(&db_path, None)
        .build()
        .await
        .map_err(|e| format!("Failed to create client: {e}"))
}

pub async fn login(
    client: &Client,
    username: &str,
    password: &str,
) -> Result<matrix_sdk::ruma::api::client::session::login::v3::Response, String> {
    client
        .matrix_auth()
        .login_username(username, password)
        .initial_device_display_name("Cosmic Matrix")
        .await
        .map_err(|e| format!("Login failed: {e}"))
}

pub fn save_session_from_client(client: &Client, homeserver: &str) -> Result<(), String> {
    let session = client
        .matrix_auth()
        .session()
        .ok_or_else(|| "No session available".to_string())?;

    let stored = StoredSession {
        homeserver: homeserver.to_string(),
        user_id: session.meta.user_id.to_string(),
        access_token: session.tokens.access_token.clone(),
        device_id: session.meta.device_id.to_string(),
    };

    config::save_session(&stored)
}

pub async fn restore_session(stored: &StoredSession) -> Result<Client, String> {
    let client = create_client(&stored.homeserver).await?;

    let user_id: OwnedUserId = stored
        .user_id
        .parse()
        .map_err(|e| format!("Invalid user_id: {e}"))?;
    let device_id: OwnedDeviceId = stored.device_id.as_str().into();

    let session = MatrixSession {
        meta: matrix_sdk::SessionMeta {
            user_id,
            device_id,
        },
        tokens: MatrixSessionTokens {
            access_token: stored.access_token.clone(),
            refresh_token: None,
        },
    };

    client
        .restore_session(session)
        .await
        .map_err(|e| format!("Session restore failed: {e}"))?;

    Ok(client)
}
