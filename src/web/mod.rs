mod events;
mod pages;
mod queries;

use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use axum::{
    Extension, Router,
    extract::{FromRef, Request, State},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing,
};
use axum_extra::extract::{PrivateCookieJar, cookie::Key};
use bb8::Pool;
use bb8_surrealdb_any::ConnectionManager;
use tera::{Context, Tera};
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::services::ServeDir;

use crate::{
    CONFIG,
    repo::{self, Db},
    web::pages::ServiceUnavailable,
};

const SESSION_COOKIE_NAME: &str = "session";

#[allow(clippy::unwrap_used)]
pub static TEMPLATES: LazyLock<RwLock<Tera>> = LazyLock::new(|| {
    let mut tera = Tera::new("templates/**/*.{j2,js}").unwrap();

    setup_tera(&mut tera);

    RwLock::new(tera)
});

#[derive(Clone)]
pub struct AppState {
    key: Key,
    pub pool: Arc<Pool<ConnectionManager<String>>>,
}

impl FromRef<AppState> for Key {
    fn from_ref(input: &AppState) -> Self {
        input.key.clone()
    }
}

#[allow(clippy::unwrap_used)]
fn t(args: &HashMap<String, tera::Value>) -> core::result::Result<tera::Value, tera::Error> {
    let locale = args.get("locale").unwrap();
    let key = args.get("key").unwrap();
    let path = std::path::Path::new(&std::env::var("PWD").unwrap())
        .join("locales")
        .join(locale.as_str().unwrap())
        .join(format!("{}.j2", key.as_str().unwrap()));

    if !path.exists() {
        return Err(format!("{} does not exist", path.to_string_lossy()).into());
    }

    let template = std::fs::read_to_string(path).unwrap();
    let template = ammonia::clean(&template);
    let text = Tera::one_off(&template, &Context::from_serialize(args).unwrap(), true).unwrap();

    Ok(text.into())
}

pub fn setup_tera(mut tera: impl std::ops::DerefMut<Target = Tera>) {
    tera.register_function("t", t);

    #[allow(clippy::unwrap_used)]
    #[cfg(debug_assertions)]
    {
        tera.add_raw_template(
            "layouts/base.j2",
            &std::fs::read_to_string(
                std::path::Path::new("templates")
                    .join("layouts")
                    .join("base.j2"),
            )
            .unwrap()
            .replace("<body", "<body data-on-load=\"@get('/.watch')\""),
        )
        .unwrap();
    }
}

async fn db_extension(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    let db = match state.pool.get_owned().await {
        Err(e) => {
            tracing::error!(?e);
            return Html(ServiceUnavailable {}.render(TEMPLATES.read().await, "en-GB"))
                .into_response();
        }
        Ok(db) => db,
    };

    if let Err(e) = db
        .use_ns(std::env::var("DB_NAMESPACE").unwrap_or_else(|_| "test".into()))
        .use_db(std::env::var("DB_DATABASE").unwrap_or_else(|_| "test".into()))
        .await
    {
        tracing::error!(?e);
        return Html(ServiceUnavailable {}.render(TEMPLATES.read().await, "en-GB")).into_response();
    }

    let db = Arc::new(db);

    request.extensions_mut().insert(db.clone());

    next.run(request).await
}

async fn authenticator(
    secrets: PrivateCookieJar,
    Extension(db): Extension<Arc<Db<'_>>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(cookie) = secrets.get(SESSION_COOKIE_NAME) else {
        return Redirect::to(&format!(
            "/login?redirect_to=/{}",
            request.uri().to_string().trim_start_matches('/')
        ))
        .into_response();
    };

    if let Err(e) = repo::auth::authenticate(&db, cookie.value()).await {
        tracing::error!(?e);
        return Redirect::to(&format!(
            "/login?redirect_to=/{}",
            request.uri().to_string().trim_start_matches('/')
        ))
        .into_response();
    }

    next.run(request).await
}

async fn normalize_trailing_slash(request: Request, next: Next) -> Response {
    let path = request.uri().path();

    if path.ends_with('/') && path != "/" {
        let new_path = path.trim_end_matches('/');
        let query = request
            .uri()
            .query()
            .map_or_else(String::new, |q| '?'.to_string() + q);
        Redirect::temporary(&format!("{new_path}{query}")).into_response()
    } else {
        next.run(request).await
    }
}

pub fn create_router(pool: Pool<ConnectionManager<String>>) -> Router {
    let state = AppState {
        key: Key::from(CONFIG.key.as_bytes()),
        pool: Arc::new(pool),
    };
    let router = Router::new()
        .route("/", routing::get(pages::public::index::get))
        .route(
            "/signup",
            routing::post(pages::public::signup::post)
                .layer(middleware::from_fn_with_state(state.clone(), db_extension))
                .get(pages::public::signup::get),
        )
        .route(
            "/login",
            routing::post(pages::public::login::post)
                .layer(middleware::from_fn_with_state(state.clone(), db_extension))
                .get(pages::public::login::get),
        )
        .merge(
            Router::new()
                .route("/app", routing::get(pages::app::home::get))
                .route_layer(middleware::from_fn_with_state(state.clone(), authenticator))
                .route_layer(middleware::from_fn_with_state(state.clone(), db_extension)),
        )
        .nest_service("/assets", ServeDir::new("assets"))
        .fallback(pages::not_found)
        .layer(middleware::from_fn(normalize_trailing_slash))
        .with_state(state);

    #[cfg(debug_assertions)]
    let router = router
        .route("/.watch", routing::get(crate::_dev::watcher))
        .layer(middleware::map_response(crate::_dev::no_cache));

    router
}

/// # Errors
pub async fn start() -> crate::Result<()> {
    let pool = crate::repo::create_pool().await?;
    let router = create_router(pool);
    let listener = TcpListener::bind(&CONFIG.listen_url).await.map_err(|e| {
        tracing::error!(?e);
        crate::Error::TcpListenerInit
    })?;

    tracing::info!("listening on http://{}", &CONFIG.listen_url);

    axum::serve(listener, router).await.map_err(|e| {
        tracing::error!(?e);
        crate::Error::ServerStart
    })?;

    Ok(())
}
