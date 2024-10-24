//! Bot main entrypoint.

#![allow(missing_docs, clippy::missing_docs_in_private_items)]

mod reconciler;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let api_id = envfury::must("TG_API_ID")?;
    let api_hash = envfury::must("TG_API_HASH")?;
    let bot_token: String = envfury::must("TG_BOT_TOKEN")?;

    let tg_channel_id = envfury::must("TG_CHANNEL_ID")?;
    let tg_channel_access_hash = envfury::must("TG_CHANNEL_ACCESS_HASH")?;

    // ---

    let params = grammers_client::InitParams {
        ..Default::default()
    };

    let session_file_path = "session.bin";

    let session = grammers_client::session::Session::load_file_or_create(session_file_path)?;

    let client_config = grammers_client::Config {
        session,
        api_id,
        api_hash,
        params,
    };

    let client = grammers_client::Client::connect(client_config).await?;

    if !client.is_authorized().await? {
        client.bot_sign_in(&bot_token).await?;
        client.session().save_to_file(session_file_path)?;
    }

    let self_user = client
        .session()
        .get_user()
        .ok_or("user not found in session after logging in")?;

    tracing::info!(?self_user, "self user loaded");

    // ---

    let reconciler = reconciler::Reconciler {
        client: client.clone(),
        chat: grammers_client::types::PackedChat {
            ty: grammers_client::session::PackedType::Megagroup,
            id: tg_channel_id,
            access_hash: Some(tg_channel_access_hash),
        },
    };

    let mut tasks = tokio::task::JoinSet::new();

    tasks.spawn(async move {
        tracing::info!(message = "running reconciler", ?reconciler);
        let Err(error) = reconciler.run().await;
        tracing::error!(message = "reconciler error", ?error);
    });

    // ---

    tokio::signal::ctrl_c().await?;

    tracing::info!(message = "saving the session before exiting");

    client.session().save_to_file(session_file_path)?;

    tasks.shutdown().await;

    Ok(())
}
