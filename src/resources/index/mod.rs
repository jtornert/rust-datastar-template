use std::convert::Infallible;

use asynk_strim::{Yielder, stream_fn};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response, Sse, sse::Event},
};
use datastar::{
    axum::ReadSignals,
    consts::ElementPatchMode,
    prelude::{PatchElements, PatchSignals},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::resources::{AppState, DatastarRequest, Render, ToJson};

#[derive(Serialize)]
struct Page {}

impl Render for Page {
    const PATH: &str = "index/page.j2";
}

#[derive(Serialize)]
struct Message {
    message: String,
}

impl Render for Message {
    const PATH: &str = "index/partials/message.j2";
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
        return Sse::new(stream_fn(
            |mut yielder: Yielder<Result<Event, Infallible>>| async move {
                let mut subscriber = nats.subscribe("messages").await.unwrap();
                while let Some(message) = subscriber.next().await {
                    yielder
                        .yield_item(Ok(PatchElements::new(
                            Message {
                                message: String::from_utf8(message.payload.to_vec()).unwrap(),
                            }
                            .render()
                            .await,
                        )
                        .mode(ElementPatchMode::Append)
                        .selector("ul")
                        .write_as_axum_sse_event()))
                        .await;
                    yielder
                        .yield_item(Ok(PatchSignals::new(
                            Signals {
                                text: String::new(),
                            }
                            .to_json(),
                        )
                        .write_as_axum_sse_event()))
                        .await;
                }
            },
        ))
        .into_response();
    }

    Html(Page {}.render().await).into_response()
}

pub async fn post(
    State(AppState { nats }): State<AppState>,
    ReadSignals(signals): ReadSignals<Signals>,
) -> impl IntoResponse {
    nats.publish("messages", signals.text.into()).await.unwrap();
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use tower::Service;

    use crate::resources::create_router;

    use super::*;

    #[tokio::test]
    async fn render() {
        Page {}.render().await;
    }

    #[tokio::test]
    async fn get_page() {
        dotenvy::dotenv().ok();
        let state = AppState::from_default_env().await.unwrap();
        let mut router = create_router(state);
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = router.call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
