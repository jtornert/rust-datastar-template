use std::convert::Infallible;

use async_fn_stream::fn_stream;
use axum::response::{IntoResponse, Response, Sse};
use notify::{RecursiveMode, Watcher, event};
use tera::Context;
use tokio::sync::mpsc;

use crate::web::{TEMPLATES, setup_tera};

#[allow(clippy::unwrap_used)]
pub async fn watcher() -> impl IntoResponse {
    Sse::new(fn_stream(|emitter| async move {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut watcher = notify::recommended_watcher(move |e| {
            tx.send(e).unwrap();
        })
        .unwrap();

        watcher
            .watch(std::path::Path::new("assets"), RecursiveMode::Recursive)
            .unwrap();
        watcher
            .watch(std::path::Path::new("locales"), RecursiveMode::Recursive)
            .unwrap();
        watcher
            .watch(std::path::Path::new("templates"), RecursiveMode::Recursive)
            .unwrap();

        while let Some(Ok(event)) = rx.recv().await {
            use datastar::prelude::ExecuteScript;
            use notify::EventKind;

            if let EventKind::Modify(event::ModifyKind::Data(_)) = event.kind {
                let path = &event.paths[0];
                let ext = path.extension().and_then(|p| p.to_str()).unwrap();
                let path = path
                    .to_str()
                    .map(|p| p.trim_start_matches(&std::env::var("PWD").unwrap()))
                    .unwrap();

                if ext == "css" {
                    let mut context = Context::new();

                    context.insert("href", path);

                    emitter
                        .emit(Ok::<_, Infallible>(
                            ExecuteScript::new(
                                TEMPLATES
                                    .read()
                                    .await
                                    .render("events/hot_reload.js", &context)
                                    .unwrap(),
                            )
                            .write_as_axum_sse_event(),
                        ))
                        .await;
                } else {
                    TEMPLATES.write().await.full_reload().unwrap();
                    setup_tera(TEMPLATES.write().await);

                    emitter
                        .emit(Ok::<_, Infallible>(
                            ExecuteScript::new(
                                TEMPLATES
                                    .read()
                                    .await
                                    .render("events/reload.js", &Context::new())
                                    .unwrap(),
                            )
                            .write_as_axum_sse_event(),
                        ))
                        .await;
                }
            }
        }
    }))
}

pub async fn no_cache(mut response: Response) -> Response {
    use axum::http::{
        HeaderValue,
        header::{CACHE_CONTROL, LAST_MODIFIED},
    };

    response.headers_mut().remove(LAST_MODIFIED);
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    response
}
