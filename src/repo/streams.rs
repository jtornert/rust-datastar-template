use std::time::Duration;

use async_nats::jetstream::{
    consumer::{Consumer, DeliverPolicy, pull},
    context::CreateStreamErrorKind,
    stream::{ConsumerErrorKind, Info, LastRawMessageErrorKind},
};
use futures_util::StreamExt;
use serde::Serialize;

use crate::{Error, repo::ISO_TIME_FORMAT};

pub const STREAM_MESSAGES: &str = "messages";
pub const STREAM_MESSAGES_SUBJECT: &str = "message";

pub async fn create_or_update_message_stream(
    ctx: &async_nats::jetstream::Context,
) -> Result<Info, async_nats::error::Error<CreateStreamErrorKind>> {
    ctx.create_or_update_stream(async_nats::jetstream::stream::Config {
        name: STREAM_MESSAGES.into(),
        allow_message_ttl: true,
        allow_direct: true,
        max_age: Duration::from_secs(10),
        subjects: vec![format!("{STREAM_MESSAGES_SUBJECT}.>")],
        ..Default::default()
    })
    .await
}

pub async fn create_message_consumer(
    ctx: &async_nats::jetstream::Context,
) -> Result<Consumer<pull::Config>, async_nats::error::Error<ConsumerErrorKind>> {
    ctx.create_consumer_on_stream(
        pull::Config {
            filter_subjects: vec![format!("{STREAM_MESSAGES_SUBJECT}.>")],
            ..Default::default()
        },
        STREAM_MESSAGES,
    )
    .await
}

#[derive(Serialize)]
pub struct Message {
    pub text: String,
    pub deadline: String,
}

pub async fn get_latest_messages(
    ctx: &async_nats::jetstream::Context,
) -> Result<Vec<Message>, Error> {
    let stream = ctx.get_stream(STREAM_MESSAGES).await.map_err(|e| {
        tracing::error!(?e);
        Error::Nats(e.to_string())
    })?;
    let last = match stream
        .get_last_raw_message_by_subject(&format!("{STREAM_MESSAGES_SUBJECT}.>"))
        .await
    {
        Ok(last) => last,
        Err(e) => {
            if LastRawMessageErrorKind::NoMessageFound == e.kind() {
                return Ok(Vec::new());
            }
            tracing::error!(?e);
            return Err(Error::Nats(e.to_string()));
        }
    };
    let from = last.sequence.saturating_sub(10).max(1);
    Ok(ctx
        .create_consumer_on_stream(
            pull::Config {
                filter_subjects: vec![format!("{STREAM_MESSAGES_SUBJECT}.>")],
                deliver_policy: DeliverPolicy::ByStartSequence {
                    start_sequence: from,
                },
                max_batch: 10,
                ..Default::default()
            },
            STREAM_MESSAGES,
        )
        .await
        .map_err(|e| {
            tracing::error!(?e);
            Error::Nats(e.to_string())
        })?
        .fetch()
        .max_messages(10)
        .messages()
        .await
        .map_err(|e| {
            tracing::error!(?e);
            Error::Nats(e.to_string())
        })?
        .filter_map(|r| match r {
            Ok(m) => {
                let text = match String::from_utf8(m.payload.to_vec()).map_err(|e| {
                    tracing::error!(?e);
                    Error::Utf8(e.to_string())
                }) {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::error!(?e);
                        return futures_util::future::ready(None);
                    }
                };
                let info = match m.info() {
                    Ok(info) => info,
                    Err(e) => {
                        tracing::error!(?e);
                        return futures_util::future::ready(None);
                    }
                };
                let deadline = match info
                    .published
                    .saturating_add(time::Duration::seconds(10))
                    .format(&ISO_TIME_FORMAT)
                {
                    Ok(deadline) => deadline,
                    Err(e) => {
                        tracing::error!(?e);
                        return futures_util::future::ready(None);
                    }
                };
                futures_util::future::ready(Some(Message { text, deadline }))
            }
            Err(e) => {
                tracing::error!(?e);
                futures_util::future::ready(None)
            }
        })
        .collect::<Vec<_>>()
        .await)
}
