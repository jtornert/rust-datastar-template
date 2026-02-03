mod index;

use std::{convert::Infallible, sync::LazyLock};

use async_nats::ConnectErrorKind;
use asynk_strim::{Yielder, stream_fn};
use axum::{
    Router,
    extract::FromRequestParts,
    http::HeaderMap,
    middleware,
    response::{Sse, sse::Event},
    routing,
};
use datastar::prelude::{ExecuteScript, PatchElements, PatchSignals};
use futures_util::Stream;
use serde::Serialize;
use tera::{Context, Tera};
use tokio::sync::RwLock;
use tower_http::{compression::CompressionLayer, services::ServeDir};

use crate::config::CONFIG;

#[allow(clippy::unwrap_used)]
static TEMPLATES: LazyLock<RwLock<Tera>> = LazyLock::new(|| {
    let mut tera = Tera::new("src/resources/**/*.j2").unwrap();
    #[cfg(debug_assertions)]
    dev::setup_hot_reload(&mut tera);
    RwLock::const_new(tera)
});

#[cfg(test)]
mod testing {
    use std::rc::Rc;

    use axum::{Router, body::Body, http::Request};
    use markup5ever_rcdom::{Node, NodeData};
    use tower::Service;

    fn walk_dom<F>(element: &Rc<Node>, callback: &mut F)
    where
        F: FnMut(&Rc<Node>),
    {
        for child in element.children.borrow().iter() {
            callback(child);
            if !child.children.borrow().is_empty() {
                walk_dom(child, callback);
            }
        }
    }

    pub fn find_anchors(element: &Rc<Node>) -> Vec<String> {
        let mut anchors = Vec::new();
        walk_dom(element, &mut |child| match &child.data {
            NodeData::Element { name, attrs, .. } => {
                if *name.local == *"a" {
                    anchors.push(
                        attrs
                            .borrow()
                            .iter()
                            .find(|a| *a.name.local == *"href")
                            .map(|a| a.value.to_string())
                            .unwrap(),
                    );
                }
            }
            _ => {}
        });
        anchors
    }

    pub async fn check_anchors(router: &mut Router, anchors: &Vec<String>) -> Result<(), String> {
        let mut errors = Vec::new();
        for anchor in anchors {
            let request = Request::builder().uri(anchor).body(Body::empty()).unwrap();
            let response = router.call(request).await.unwrap();
            if (200..=399).contains(&response.status().as_u16()) {
                errors.push(anchor.to_owned());
            }
        }
        if !errors.is_empty() {
            return Err(format!("broken links: {}", anchors.join(", ")));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test_server {
    use std::{process::Child, sync::LazyLock};

    use rand::Rng;
    use tokio::sync::OnceCell;

    pub static NATS_PORT: LazyLock<u16> =
        LazyLock::new(|| rand::rng().random_range(1024..u16::MAX));
    pub static NATS_TEST_SERVER: OnceCell<Child> = OnceCell::const_new();
}

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

    #[cfg(test)]
    pub async fn test_state() -> Result<Self, async_nats::error::Error<ConnectErrorKind>> {
        test_server::NATS_TEST_SERVER
            .get_or_init(|| async {
                use std::{
                    process::{Command, Stdio},
                    time::Duration,
                };

                let child = Command::new("nats-server")
                    .args([
                        "--jetstream",
                        "--port",
                        &*test_server::NATS_PORT.to_string(),
                    ])
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(500)).await;
                child
            })
            .await;
        Ok(Self {
            nats: async_nats::connect(format!("127.0.0.1:{}", *test_server::NATS_PORT))
                .await
                .unwrap(),
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

pub fn datastar_sse<F: Future<Output = ()> + Send>(
    f: impl FnOnce(DatastarSse) -> F + Send + 'static,
) -> Sse<impl Stream<Item = Result<Event, Infallible>> + Send> {
    Sse::new(stream_fn(
        |yielder: Yielder<Result<Event, Infallible>>| async move {
            f(DatastarSse(yielder)).await;
        },
    ))
}

pub struct DatastarSse(Yielder<Result<Event, Infallible>>);

#[allow(dead_code)]
impl DatastarSse {
    pub async fn patch_elements(&mut self, patch: PatchElements) {
        self.0.yield_item(Ok(patch.write_as_axum_sse_event())).await;
    }

    pub async fn patch_signals(&mut self, patch: PatchSignals) {
        self.0.yield_item(Ok(patch.write_as_axum_sse_event())).await;
    }

    pub async fn execute_script(&mut self, script: ExecuteScript) {
        self.0
            .yield_item(Ok(script.write_as_axum_sse_event()))
            .await;
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
    router.layer(middleware::from_fn(compression::compress_sse))
}

mod compression {
    //! taken from <https://github.com/tokio-rs/axum/discussions/2728#discussioncomment-11919208>

    use std::{
        io::Write,
        pin::{Pin, pin},
        task::{Context, Poll},
    };

    use axum::{
        body::{Body, BodyDataStream, Bytes},
        extract::Request,
        http::{
            HeaderValue,
            header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, VARY},
        },
        middleware::Next,
        response::Response,
    };
    use brotli::CompressorWriter;
    use futures_util::Stream;

    pub async fn compress_sse(request: Request, next: Next) -> Response {
        let accept_encoding = request.headers().get(ACCEPT_ENCODING).cloned();

        let response = next.run(request).await;

        let content_encoding = response.headers().get(CONTENT_ENCODING);
        let content_type = response.headers().get(CONTENT_TYPE);

        // No accept-encoding from client or content-type from server.
        let (Some(ct), Some(ae)) = (content_type, accept_encoding) else {
            return response;
        };
        // Already compressed.
        if content_encoding.is_some() {
            return response;
        }
        // Not text/event-stream.
        if ct.as_bytes() != b"text/event-stream" {
            return response;
        }
        // Client doesn't accept brotli compression.
        if !ae.to_str().map(|v| v.contains("br")).unwrap_or(false) {
            return response;
        }

        let (mut parts, body) = response.into_parts();

        let body = body.into_data_stream();
        let body = Body::from_stream(CompressedStream::new(body));

        parts
            .headers
            .insert(CONTENT_ENCODING, HeaderValue::from_static("br"));
        parts
            .headers
            .insert(VARY, HeaderValue::from_static("accept-encoding"));

        Response::from_parts(parts, body)
    }

    struct CompressedStream {
        inner: BodyDataStream,
        compression: CompressorWriter<Vec<u8>>,
    }

    impl CompressedStream {
        pub fn new(body: BodyDataStream) -> Self {
            Self {
                inner: body,
                compression: CompressorWriter::new(Vec::new(), 4096, 11, 22),
            }
        }
    }

    impl Stream for CompressedStream {
        type Item = Result<Bytes, axum::Error>;

        #[inline]
        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            match pin!(&mut self.inner).as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(x))) => {
                    self.compression.write_all(&x).unwrap();
                    self.compression.flush().unwrap();

                    let mut buf = Vec::new();
                    std::mem::swap(&mut buf, self.compression.get_mut());

                    Poll::Ready(Some(Ok(buf.into())))
                }
                x => x,
            }
        }
    }
}

#[cfg(debug_assertions)]
#[allow(clippy::unwrap_used, clippy::case_sensitive_file_extension_comparisons)]
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
        if response.status() == StatusCode::NOT_FOUND {
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
                    let Ok(event @ notify::Event {
                        kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(_)),
                        ..
                    }) = result else {
                        continue;
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
                            let mut tera = TEMPLATES
                                .write()
                                .await;
                            tera
                                .add_raw_template(
                                &path
                                    .strip_prefix("src/resources/")
                                    .unwrap()
                                    .to_string_lossy(),
                                &std::fs::read_to_string(path).unwrap(),
                                )
                                .unwrap();
                            setup_hot_reload(&mut tera);
                            drop(tera);
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
                                        "(()=>{{const n='{}',e=document.querySelector(`link[href^='${{n}}']`);if(!e)return;const t=e.cloneNode(!0);t.href=`${{n}}?t=${{new Date().valueOf()}}`,document.head.append(t),e.remove()}})();",
                                        path.to_str().unwrap().trim_start_matches("src")
                                    )).write_as_axum_sse_event()
                                )).await;
                                }
                            } else if let Ok(path) = path.strip_prefix("src/resources") {
                                yielder.yield_item(Ok(ExecuteScript::new(
                                    format!(
                                        "(()=>{{const n='/assets/pages/{}',e=document.querySelector(`link[href^='${{n}}']`);if(!e)return;const t=e.cloneNode(!0);t.href=`${{n}}?t=${{new Date().valueOf()}}`,document.head.append(t),e.remove()}})();",
                                        path.to_str().unwrap()
                                    )).write_as_axum_sse_event()
                                )).await;
                            }
                        }
                        "ts" => {
                            yielder.yield_item(Ok(ExecuteScript::new("location.reload()").write_as_axum_sse_event())).await;
                        }
                        _ => {}
                    }
                }
            },
        ))
        .into_response();
    }
}
