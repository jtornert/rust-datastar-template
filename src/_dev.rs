use std::convert::Infallible;

use axum::response::{IntoResponse, Response, Sse};

use crate::web::{TEMPLATES, setup_tera};

#[allow(clippy::unwrap_used)]
pub async fn watcher() -> impl IntoResponse {
    use async_stream::stream;
    use notify::{RecursiveMode, Watcher, event};
    use tokio::sync::mpsc;

    Sse::new(stream! {
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
                let path = path.to_str()
                .map(|p| p.trim_start_matches(&std::env::var("PWD").unwrap()))
                .unwrap();

                match ext {
                    "j2" => {
                        TEMPLATES.write().await.full_reload().unwrap();
                        setup_tera(TEMPLATES.write().await);

                        yield Ok::<_, Infallible>(ExecuteScript::new(
                            std::fs::read_to_string(
                                std::path::Path::new("templates").join("events")
                                    .join("reload.js")
                                )
                                .unwrap(),
                        )
                        .write_as_axum_sse_event());
                    }
                    "js" => {
                        yield Ok::<_, Infallible>(ExecuteScript::new(
                            std::fs::read_to_string(
                                std::path::Path::new("templates").join("events")
                                    .join("reload.js")
                            )
                                .unwrap(),
                        )
                        .write_as_axum_sse_event());
                    }
                    _ => {
                        use tera::Context;

                        let mut context = Context::new();

                        context.insert(
                            "href",
                            path,
                        );

                        yield Ok::<_, Infallible>(ExecuteScript::new(TEMPLATES
                            .read()
                            .await
                            .render("hot_reload.js", &context)
                            .unwrap()).write_as_axum_sse_event());
                    }
                }
            }
        }
    })
}

pub async fn no_cache(mut r: Response) -> Response {
    use axum::http::{
        HeaderValue,
        header::{CACHE_CONTROL, LAST_MODIFIED},
    };

    r.headers_mut().remove(LAST_MODIFIED);
    r.headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    r
}
