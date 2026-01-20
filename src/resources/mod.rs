mod index;

use std::{convert::Infallible, sync::LazyLock};

use async_nats::ConnectErrorKind;
use axum::{Router, extract::FromRequestParts, http::HeaderMap, routing};
use serde::Serialize;
use tera::{Context, Tera};
use tokio::sync::RwLock;
use tower_http::{compression::CompressionLayer, services::ServeDir};

use crate::config::CONFIG;

static TEMPLATES: LazyLock<RwLock<Tera>> = LazyLock::new(|| {
    let mut tera = Tera::new("src/resources/**/*.j2").unwrap();
    #[cfg(debug_assertions)]
    dev::setup_hot_reload(&mut tera);
    RwLock::const_new(tera)
});

#[derive(Clone)]
pub struct AppState {
    pub nats: async_nats::Client,
}

impl AppState {
    pub async fn from_default_env() -> Result<Self, async_nats::error::Error<ConnectErrorKind>> {
        Ok(Self {
            nats: async_nats::connect(&CONFIG.nats_servers).await?,
        })
    }
}

pub struct DatastarRequest(pub bool);

impl<S> FromRequestParts<S> for DatastarRequest
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            HeaderMap::from_request_parts(parts, state)
                .await
                .unwrap()
                .get("datastar-request")
                .is_some_and(|h| h.as_bytes() == b"true"),
        ))
    }
}

pub trait Render {
    const PATH: &str;

    async fn render(&self) -> String
    where
        Self: Serialize,
    {
        let context = Context::from_serialize(self).unwrap();
        TEMPLATES.read().await.render(Self::PATH, &context).unwrap()
    }
}

pub trait ToJson
where
    Self: Serialize,
{
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl<S> ToJson for S where S: Serialize {}

pub fn create_router(state: AppState) -> Router {
    let router = Router::new()
        .route("/", routing::get(index::get).post(index::post))
        .with_state(state);
    #[cfg(debug_assertions)]
    let router = router
        .nest_service("/assets", ServeDir::new("public"))
        .layer(axum::middleware::from_fn(dev::handle_assets));
    #[cfg(not(debug_assertions))]
    let router = router.nest_service(
        "/assets",
        ServeDir::new("dist/assets").fallback(ServeDir::new("public")),
    );
    let router = router.layer(
        CompressionLayer::new()
            .br(true)
            .quality(tower_http::CompressionLevel::Best),
    );
    #[cfg(debug_assertions)]
    let router = router
        .route("/.hotreload", routing::get(dev::hot_reload))
        .layer(axum::middleware::map_response(dev::no_cache));
    router
}

#[cfg(debug_assertions)]
mod dev {
    use std::{
        convert::Infallible,
        path::Path,
        sync::LazyLock,
        time::{SystemTime, UNIX_EPOCH},
    };

    use asynk_strim::{Yielder, stream_fn};
    use axum::{
        extract::{Query, Request},
        http::{
            StatusCode,
            header::{CONTENT_LENGTH, CONTENT_TYPE, LAST_MODIFIED},
        },
        middleware::Next,
        response::{IntoResponse, Response, Sse, sse::Event},
    };
    use datastar::prelude::ExecuteScript;
    use notify::{RecursiveMode, Watcher};
    use serde::Deserialize;
    use tera::Tera;

    use crate::resources::TEMPLATES;

    static DEV_BUILD_TIMESTAMP: LazyLock<u64> = LazyLock::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    pub fn setup_hot_reload(tera: &mut Tera) {
        tera.add_raw_template(
            "base.j2",
            &std::fs::read_to_string("src/resources/base.j2")
                .unwrap()
                .replace(
                    "<body",
                    &format!(
                        r#"<body data-init="@get('/.hotreload?t={}',{{retryScaler:1,retryMaxCount:60}})""#,
                        *DEV_BUILD_TIMESTAMP
                    ),
                ),
        )
        .unwrap();
    }

    pub async fn handle_assets(request: Request, next: Next) -> Response {
        let path = request.uri().path().to_owned();
        let response = next.run(request).await;
        if let StatusCode::NOT_FOUND = response.status() {
            if path.starts_with("/assets/pages") {
                if path.ends_with(".css") {
                    let file_provider = lightningcss::bundler::FileProvider::new();
                    let mut bundler = lightningcss::bundler::Bundler::new(
                        &file_provider,
                        None,
                        lightningcss::stylesheet::ParserOptions::default(),
                    );
                    let input = Path::new("src").join(&path.replace("/assets/pages", "resources"));
                    let stylesheet = bundler.bundle(&input).unwrap();
                    let result = stylesheet
                        .to_css(lightningcss::printer::PrinterOptions::default())
                        .unwrap();
                    return Response::builder()
                        .header(CONTENT_TYPE, mime::TEXT_CSS_UTF_8.essence_str())
                        .header(CONTENT_LENGTH, result.code.len())
                        .body(result.code.into())
                        .unwrap();
                } else if path.ends_with(".js") {
                    let input = Path::new("src")
                        .join(path.replace("/assets/pages", "resources"))
                        .with_extension("ts");
                    let mut options_builder = esbuild_rs::BuildOptionsBuilder::new();
                    options_builder
                        .entry_points
                        .push(input.to_str().unwrap().into());
                    options_builder.bundle = true;
                    options_builder.format = esbuild_rs::Format::ESModule;
                    let result = esbuild_rs::build(options_builder.build())
                        .await
                        .output_files;
                    let content = result.as_slice().first().unwrap().data.as_str().to_owned();
                    return Response::builder()
                        .header(
                            CONTENT_TYPE,
                            mime::APPLICATION_JAVASCRIPT_UTF_8.essence_str(),
                        )
                        .header(CONTENT_LENGTH, content.len())
                        .body(content.into())
                        .unwrap();
                }
                tracing::debug!("generate page content {:?}", Path::new(&path));
            } else if path.starts_with("/assets") {
                if path.ends_with(".css") {
                    let file_provider = lightningcss::bundler::FileProvider::new();
                    let mut bundler = lightningcss::bundler::Bundler::new(
                        &file_provider,
                        None,
                        lightningcss::stylesheet::ParserOptions::default(),
                    );
                    let input = Path::new("src").join(&path[1..]);
                    let stylesheet = bundler.bundle(&input).unwrap();
                    let result = stylesheet
                        .to_css(lightningcss::printer::PrinterOptions::default())
                        .unwrap();
                    return Response::builder()
                        .header(CONTENT_TYPE, mime::TEXT_CSS_UTF_8.essence_str())
                        .header(CONTENT_LENGTH, result.code.len())
                        .body(result.code.into())
                        .unwrap();
                } else if path.ends_with(".js") {
                    let input = Path::new("src")
                        .join(path.replace("/assets/js", "assets/ts"))
                        .with_extension("ts");
                    let mut options_builder = esbuild_rs::BuildOptionsBuilder::new();
                    options_builder
                        .entry_points
                        .push(input.to_str().unwrap().into());
                    options_builder.bundle = true;
                    options_builder.format = esbuild_rs::Format::ESModule;
                    let result = esbuild_rs::build(options_builder.build())
                        .await
                        .output_files;
                    let content = result.as_slice().first().unwrap().data.as_str().to_owned();
                    return Response::builder()
                        .header(
                            CONTENT_TYPE,
                            mime::APPLICATION_JAVASCRIPT_UTF_8.essence_str(),
                        )
                        .header(CONTENT_LENGTH, content.len())
                        .body(content.into())
                        .unwrap();
                }
                tracing::debug!("generate global content {:?}", Path::new(&path));
            }
        }
        response
    }

    pub async fn no_cache(mut response: Response) -> Response {
        let headers = response.headers_mut();
        headers.remove(LAST_MODIFIED);
        response
    }

    #[derive(Deserialize)]
    pub(super) struct TQuery {
        t: u64,
    }

    pub async fn hot_reload(Query(TQuery { t }): Query<TQuery>) -> Response {
        if t != *DEV_BUILD_TIMESTAMP {
            return Sse::new(stream_fn(
                |mut yielder: Yielder<Result<Event, Infallible>>| async move {
                    yielder
                        .yield_item(Ok(
                            ExecuteScript::new("location.reload()").write_as_axum_sse_event()
                        ))
                        .await;
                },
            ))
            .into_response();
        }
        return Sse::new(stream_fn(
            |mut yielder: Yielder<Result<Event, Infallible>>| async move {
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                let mut watcher = notify::recommended_watcher(move |result| {
                    tx.send(result).unwrap();
                })
                .unwrap();
                watcher
                    .watch(
                        &Path::new("src").join("resources"),
                        RecursiveMode::Recursive,
                    )
                    .unwrap();
                watcher
                    .watch(&Path::new("src").join("assets"), RecursiveMode::Recursive)
                    .unwrap();
                while let Some(result) = rx.recv().await {
                    let event = match result {
                        Ok(
                            event @ notify::Event {
                                kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(_)),
                                ..
                            },
                        ) => event,
                        _ => continue,
                    };
                    let Some(path) = event
                        .paths
                        .first()
                        .and_then(|p| p.strip_prefix(env!("CARGO_MANIFEST_DIR")).ok())
                    else {
                        continue;
                    };
                    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                        continue;
                    };
                    match ext {
                        "j2" => {
                            let mut tera = TEMPLATES.write().await;
                            tera.add_raw_template(
                                &path
                                    .strip_prefix("src/resources/")
                                    .unwrap()
                                    .to_string_lossy(),
                                &std::fs::read_to_string(path).unwrap(),
                            )
                            .unwrap();
                            yielder
                                .yield_item(Ok(ExecuteScript::new("location.reload()")
                                    .write_as_axum_sse_event()))
                                .await;
                        }
                        "css" => {
                            if path.starts_with("src/assets") {
                                if path.file_name().and_then(|f|f.to_str()).is_some_and(|f|f.starts_with('_')) {
                                    yielder.yield_item(Ok(ExecuteScript::new("location.reload()").write_as_axum_sse_event())).await;
                                } else {
                                    yielder.yield_item(Ok(ExecuteScript::new(
                                    format!(
                                        r#"(()=>{{const n='{}',e=document.querySelector(`link[href^='${{n}}']`);if(!e)return;const t=e.cloneNode(!0);t.href=`${{n}}?t=${{new Date().valueOf()}}`,document.head.append(t),e.remove()}})();"#,
                                        path.to_str().unwrap().trim_start_matches("src")
                                    )).write_as_axum_sse_event()
                                )).await;
                                }
                            } else if let Ok(path) = path.strip_prefix("src/resources") {
                                yielder.yield_item(Ok(ExecuteScript::new(
                                    format!(
                                        r#"(()=>{{const n='{}',e=document.querySelector(`link[href^='${{n}}']`);if(!e)return;const t=e.cloneNode(!0);t.href=`${{n}}?t=${{new Date().valueOf()}}`,document.head.append(t),e.remove()}})();"#,
                                        format!("/assets/pages/{}", path.to_str().unwrap())
                                    )).write_as_axum_sse_event()
                                )).await;
                            }
                        }
                        "ts" => {
                            yielder.yield_item(Ok(ExecuteScript::new("location.reload()").write_as_axum_sse_event())).await;
                        }
                        _ => {
                            continue;
                        }
                    }
                }
            },
        ))
        .into_response();
    }
}
