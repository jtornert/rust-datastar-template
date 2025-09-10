pub mod app;
pub mod public;

use axum::response::{Html, IntoResponse};
use serde::Serialize;
use tera_template_macro::TeraTemplate;
use uuid::Uuid;

use crate::web::{DEFAULT_LOCALE, TEMPLATES};

#[derive(Serialize, TeraTemplate)]
#[template(path = "pages/not_found.j2")]
struct NotFound {}

pub async fn not_found() -> impl IntoResponse {
    Html(NotFound {}.render(TEMPLATES.read().await, DEFAULT_LOCALE))
}

#[derive(Serialize, TeraTemplate)]
#[template(path = "pages/service_unavailable.j2")]
pub struct ServiceUnavailable {
    pub uuid: Uuid,
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::Service;

    use crate::{
        _test::{TestResult, create_test_pool, setup_test},
        web::{DEFAULT_LOCALE, TEMPLATES, create_router, pages::NotFound},
    };

    #[tokio::test]
    async fn get_not_found() -> TestResult {
        setup_test();

        let pool = create_test_pool().await?;
        let mut router = create_router(pool);
        let request = Request::builder()
            .uri("/non-existent")
            .body(Body::empty())?;
        let response = router.call(request).await?;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            axum::body::to_bytes(response.into_body(), usize::MAX).await?,
            NotFound {}.render(TEMPLATES.read().await, DEFAULT_LOCALE)
        );

        Ok(())
    }
}
