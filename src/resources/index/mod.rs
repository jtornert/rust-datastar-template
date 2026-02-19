use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use datastar::{
    axum::ReadSignals,
    prelude::{PatchElements, PatchSignals},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::{
    repo::{self, streams::Message},
    resources::{AppState, DatastarRequest, Render, ToJson, datastar_sse},
};

#[derive(Serialize)]
struct Page {
    messages: Vec<Message>,
}

impl Render for Page {
    const PATH: &str = "index/page.j2";
}

#[derive(Serialize, Deserialize)]
pub struct Signals {
    text: String,
}

pub async fn get(
    State(AppState { nats }): State<AppState>,
    DatastarRequest(datastar_request): DatastarRequest,
) -> Response {
    let ctx = async_nats::jetstream::new(nats);
    if datastar_request {
        if let Err(e) = repo::streams::create_or_update_message_stream(&ctx).await {
            tracing::error!(?e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        let consumer = match repo::streams::create_message_consumer(&ctx).await {
            Ok(consumer) => consumer,
            Err(e) => {
                tracing::error!(?e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        let mut subscriber = match consumer.messages().await {
            Ok(subscriber) => subscriber,
            Err(e) => {
                tracing::error!(?e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        return datastar_sse(|mut sse| async move {
            while let Some(message) = subscriber.next().await {
                if let Err(e) = message {
                    tracing::error!(?e);
                    continue;
                }
                let messages = match repo::streams::get_latest_messages(&ctx).await {
                    Ok(messages) => messages,
                    Err(e) => {
                        tracing::error!(?e);
                        // TODO add notification
                        continue;
                    }
                };
                sse.patch_elements(PatchElements::new(Page { messages }.render().await))
                    .await;
                sse.patch_signals(PatchSignals::new(
                    Signals {
                        text: String::new(),
                    }
                    .to_json(),
                ))
                .await;
            }
        })
        .into_response();
    }

    Html(
        Page {
            messages: match repo::streams::get_latest_messages(&ctx).await {
                Ok(messages) => messages,
                Err(e) => {
                    tracing::error!(?e);
                    Vec::new()
                }
            },
        }
        .render()
        .await,
    )
    .into_response()
}

pub async fn post(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(AppState { nats }): State<AppState>,
    ReadSignals(signals): ReadSignals<Signals>,
) -> impl IntoResponse {
    let ctx = async_nats::jetstream::new(nats);
    let message = signals.text.trim().to_owned();
    if !message.is_empty() {
        if let Err(e) = ctx
            .publish(
                format!("{}.{}", repo::streams::STREAM_MESSAGES_SUBJECT, addr),
                message.into(),
            )
            .await
        {
            tracing::error!(?e);
        }
    }
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {

    use std::time::{SystemTime, UNIX_EPOCH};

    use axum::{body::Body, http::Request};

    use html5ever::{
        ParseOpts, parse_document,
        tendril::{TendrilSink, fmt::Slice},
    };
    use markup5ever_rcdom::RcDom;
    use tower::Service;

    use crate::resources::{create_router, testing};

    use super::*;

    #[tokio::test]
    async fn render() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let page = Page {
            messages: vec![Message {
                text: now.to_string(),
                deadline: (now + 1).to_string(),
            }],
        }
        .render()
        .await;
        assert!(page.contains(&now.to_string()));
        assert!(page.contains(&(now + 1).to_string()));
    }

    #[tokio::test]
    async fn get_page() {
        dotenvy::dotenv().ok();
        let server = testing::start_test_server();
        let state = AppState::test_state(&server).await.unwrap();
        let mut router = create_router(state);
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = router.call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let dom = parse_document(RcDom::default(), ParseOpts::default())
            .from_utf8()
            .read_from(&mut body.as_bytes())
            .unwrap();
        let anchors = testing::find_anchors(&dom.document);
        testing::check_anchors(&mut router, &anchors).await.unwrap();
    }
}
