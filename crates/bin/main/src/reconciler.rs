use std::sync::Arc;

use grammers_client::grammers_tl_types as tl;

#[derive(Debug)]
pub struct Reconciler {
    pub client: grammers_client::Client,
}

impl Reconciler {
    pub async fn run(self) -> Result<core::convert::Infallible, crate::Error> {
        let mut tasks = tokio::task::JoinSet::new();

        let this = Arc::new(self);

        tasks.spawn({
            let this = Arc::clone(&this);
            async move {
                loop {
                    let (update, chats) = this.client.next_raw_update().await.unwrap();
                    this.handle_update(update, chats).await;
                }
            }
        });

        tasks.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(
                1, /* first time wait less */
            ))
            .await;

            loop {
                this.reconcile().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });

        let res = tasks.join_next().await.unwrap().unwrap();
        tasks.shutdown().await;
        res
    }

    async fn handle_update(
        &self,
        update: tl::enums::Update,
        _chats: Arc<grammers_client::ChatMap>,
    ) {
        tracing::info!(message = "got update", ?update);
    }

    async fn reconcile(&self) -> Result<(), crate::Error> {
        let mut iter = self
            .client
            .iter_participants(grammers_client::types::PackedChat {
                ty: grammers_client::session::PackedType::Megagroup,
                // The data here is irrelevant - in fact the stall happens before they begin
                // to matter.
                // So we use some known-bogus data here to simplify the setup for testers.
                id: 1234,
                access_hash: Some(5678),
            });

        tracing::info!(message = "before iter next", note = "<----- hangs here");

        let maybe_first = iter.next().await?;

        tracing::info!(message = "after iter next", ?maybe_first);

        Ok(())
    }
}
