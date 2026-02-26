use std::sync::Arc;

use cosmic::iced::futures::SinkExt;
use cosmic::iced::stream;
use cosmic::iced::Subscription;
use futures::StreamExt;
use matrix_sdk::encryption::verification::{SasState, Verification, VerificationRequestState};
use matrix_sdk::ruma::api::client::uiaa::{AuthData, Password, UserIdentifier};
use matrix_sdk::ruma::events::key::verification::VerificationMethod;
use matrix_sdk::ruma::OwnedUserId;
use matrix_sdk::Client;

use crate::message::{CrossSigningStatus, Message, VerificationStateUpdate};

pub async fn bootstrap_cross_signing(
    client: Client,
    user_id: String,
    password: Option<String>,
) -> Message {
    // 1. Try without auth; capture UIAA session if required
    let session = match client.encryption().bootstrap_cross_signing_if_needed(None).await {
        Ok(()) => return Message::CrossSigningBootstrapped,
        Err(e) => {
            if let Some(uiaa) = e.as_uiaa_response() {
                uiaa.session.clone()
            } else {
                return Message::CrossSigningBootstrapFailed(e.to_string());
            }
        }
    };

    // 2. Retry with password
    let Some(pw) = password else {
        return Message::CrossSigningBootstrapFailed(
            "UIA required but no password in memory".into(),
        );
    };

    let localpart = user_id
        .split(':')
        .next()
        .and_then(|s| s.strip_prefix('@'))
        .unwrap_or(&user_id)
        .to_string();

    let mut pass = Password::new(UserIdentifier::UserIdOrLocalpart(localpart), pw);
    pass.session = session;

    match client
        .encryption()
        .bootstrap_cross_signing_if_needed(Some(AuthData::Password(pass)))
        .await
    {
        Ok(()) => Message::CrossSigningBootstrapped,
        Err(e) => Message::CrossSigningBootstrapFailed(e.to_string()),
    }
}

pub async fn fetch_cross_signing_status(client: Client) -> Message {
    let status = client.encryption().cross_signing_status().await;
    let cs = match status {
        Some(s) if s.has_master && s.has_self_signing && s.has_user_signing => {
            CrossSigningStatus::Verified
        }
        Some(_) => CrossSigningStatus::Unverified,
        None => CrossSigningStatus::Unknown,
    };
    Message::CrossSigningStatusFetched(cs)
}

pub async fn start_self_verification(client: Client, own_user_id: OwnedUserId) -> Message {
    let identity = match client.encryption().get_user_identity(&own_user_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return Message::CrossSigningBootstrapFailed(
                "Own identity not found on server".into(),
            )
        }
        Err(e) => return Message::CrossSigningBootstrapFailed(e.to_string()),
    };
    match identity
        .request_verification_with_methods(vec![VerificationMethod::SasV1])
        .await
    {
        Ok(req) => Message::VerificationRequestCreated(req.flow_id().to_owned()),
        Err(e) => Message::CrossSigningBootstrapFailed(e.to_string()),
    }
}

pub async fn accept_incoming_verification(
    client: Client,
    sender: OwnedUserId,
    flow_id: String,
) -> Message {
    let req = match client
        .encryption()
        .get_verification_request(&sender, &flow_id)
        .await
    {
        Some(r) => r,
        None => {
            return Message::CrossSigningBootstrapFailed("Verification request not found".into())
        }
    };
    if let Err(e) = req.accept().await {
        return Message::CrossSigningBootstrapFailed(e.to_string());
    }
    // start_sas() may return None if the other side drives; subscription handles Transitioned
    let _ = req.start_sas().await;
    Message::VerificationRequestCreated(flow_id)
}

pub async fn confirm_verification(
    client: Client,
    user_id: OwnedUserId,
    flow_id: String,
) -> Message {
    if let Some(v) = client.encryption().get_verification(&user_id, &flow_id).await {
        if let Some(sas) = v.sas() {
            let _ = sas.confirm().await;
        }
    }
    Message::None
}

pub async fn mismatch_verification(
    client: Client,
    user_id: OwnedUserId,
    flow_id: String,
) -> Message {
    if let Some(v) = client.encryption().get_verification(&user_id, &flow_id).await {
        if let Some(sas) = v.sas() {
            let _ = sas.mismatch().await;
        }
    }
    Message::None
}

pub async fn cancel_verification(
    client: Client,
    user_id: OwnedUserId,
    flow_id: String,
) -> Message {
    if let Some(v) = client.encryption().get_verification(&user_id, &flow_id).await {
        if let Some(sas) = v.sas() {
            let _ = sas.cancel().await;
        }
    }
    if let Some(r) = client
        .encryption()
        .get_verification_request(&user_id, &flow_id)
        .await
    {
        let _ = r.cancel().await;
    }
    Message::VerificationStateChanged(VerificationStateUpdate::Cancelled(
        "Cancelled by user".into(),
    ))
}

struct VerificationSubscriptionMarker;

pub fn verification_subscription(
    client: Arc<Client>,
    own_user_id: OwnedUserId,
    flow_id: String,
) -> Subscription<Message> {
    let id = (
        std::any::TypeId::of::<VerificationSubscriptionMarker>(),
        flow_id.clone(),
    );
    Subscription::run_with_id(
        id,
        stream::channel(32, move |mut output| {
            let client = client.clone();
            async move {
                run_verification_stream(client, own_user_id, flow_id, &mut output).await;
                futures::future::pending::<()>().await;
            }
        }),
    )
}

async fn run_verification_stream(
    client: Arc<Client>,
    own_user_id: OwnedUserId,
    flow_id: String,
    output: &mut futures::channel::mpsc::Sender<Message>,
) {
    let request = match client
        .encryption()
        .get_verification_request(&own_user_id, &flow_id)
        .await
    {
        Some(r) => r,
        None => return,
    };

    // Phase 1: wait for Ready state, then start SAS
    let mut req_changes = request.changes();
    let sas = loop {
        match req_changes.next().await {
            Some(VerificationRequestState::Ready { .. }) => {
                let _ = output
                    .send(Message::VerificationStateChanged(
                        VerificationStateUpdate::Accepted,
                    ))
                    .await;
                match request.start_sas().await {
                    Ok(Some(sas)) => break sas,
                    Ok(None) => {
                        // Other side is driving; wait for Transitioned
                        continue;
                    }
                    Err(e) => {
                        let _ = output
                            .send(Message::VerificationStateChanged(
                                VerificationStateUpdate::Cancelled(e.to_string()),
                            ))
                            .await;
                        return;
                    }
                }
            }
            Some(VerificationRequestState::Transitioned {
                verification: Verification::SasV1(sas),
            }) => {
                let _ = output
                    .send(Message::VerificationStateChanged(
                        VerificationStateUpdate::Accepted,
                    ))
                    .await;
                break sas;
            }
            Some(VerificationRequestState::Done) => {
                let _ = output
                    .send(Message::VerificationStateChanged(
                        VerificationStateUpdate::Done,
                    ))
                    .await;
                return;
            }
            Some(VerificationRequestState::Cancelled(info)) => {
                let _ = output
                    .send(Message::VerificationStateChanged(
                        VerificationStateUpdate::Cancelled(info.reason().to_string()),
                    ))
                    .await;
                return;
            }
            Some(_) | None => continue,
        }
    };

    // Phase 2: drive SAS state changes
    let mut sas_changes = sas.changes();
    loop {
        match sas_changes.next().await {
            Some(SasState::KeysExchanged { emojis, decimals: _ }) => {
                if let Some(emoji_str) = emojis {
                    let v: Vec<(String, String)> = emoji_str
                        .emojis
                        .iter()
                        .map(|e| (e.symbol.to_owned(), e.description.to_owned()))
                        .collect();
                    let _ = output
                        .send(Message::VerificationStateChanged(
                            VerificationStateUpdate::EmojiReady(v),
                        ))
                        .await;
                }
            }
            Some(SasState::Done { .. }) => {
                let _ = output
                    .send(Message::VerificationStateChanged(
                        VerificationStateUpdate::Done,
                    ))
                    .await;
                return;
            }
            Some(SasState::Cancelled(info)) => {
                let _ = output
                    .send(Message::VerificationStateChanged(
                        VerificationStateUpdate::Cancelled(info.reason().to_string()),
                    ))
                    .await;
                return;
            }
            Some(_) | None => {}
        }
    }
}
