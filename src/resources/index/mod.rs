use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use datastar::{
    axum::ReadSignals,
    prelude::{PatchElements, PatchSignals},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::resources::{AppState, DatastarRequest, Render, ToJson, datastar_sse};

#[derive(Serialize)]
struct Page {
    text: Option<String>,
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
    if datastar_request {
        let mut subscriber = match nats.subscribe("messages").await {
            Ok(subscriber) => subscriber,
            Err(e) => {
                tracing::error!(?e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        return datastar_sse(|mut sse| async move {
            while let Some(message) = subscriber.next().await {
                let text = match String::from_utf8(message.payload.to_vec()) {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::error!(?e);
                        continue;
                    }
                };
                sse.patch_elements(PatchElements::new(Page { text: Some(text) }.render().await))
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

    Html(Page { text: None }.render().await).into_response()
}

pub async fn post(
    State(AppState { nats }): State<AppState>,
    ReadSignals(signals): ReadSignals<Signals>,
) -> impl IntoResponse {
    if let Err(e) = nats
        .publish("messages", signals.text.trim().to_owned().into())
        .await
    {
        tracing::error!(?e);
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
            text: Some(now.to_string()),
        }
        .render()
        .await;
        assert!(page.contains(&now.to_string()));
    }

    #[tokio::test]
    async fn get_page() {
        dotenvy::dotenv().ok();
        let state = AppState::test_state().await.unwrap();
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
