pub mod home {
    use std::sync::Arc;

    use axum::{
        Extension,
        response::{Html, IntoResponse},
    };
    use serde::Serialize;
    use tera_template_macro::TeraTemplate;

    use crate::{
        models::user::User,
        repo::{self, Db, DbRecord},
        web::{TEMPLATES, pages::ServiceUnavailable},
    };

    #[derive(Serialize, TeraTemplate)]
    #[template(path = "pages/app/home.j2")]
    struct Page {
        user: DbRecord<User>,
    }

    pub async fn get(Extension(db): Extension<Arc<Db<'_>>>) -> impl IntoResponse {
        let user = match repo::user::me(&db).await {
            Err(_) => {
                return Html(ServiceUnavailable {}.render(TEMPLATES.read().await, "en-GB"));
            }
            Ok(user) => user,
        };

        Html(Page { user }.render(TEMPLATES.read().await, "en-GB"))
    }

    #[cfg(test)]
    mod tests {
        use axum::{
            body::Body,
            http::{
                Method, Request, StatusCode,
                header::{CONTENT_TYPE, COOKIE, SET_COOKIE},
            },
        };
        use tower::Service;

        use crate::{
            _test::{TestResult, create_test_pool, credentials, setup_test},
            repo,
            web::create_router,
        };

        #[tokio::test]
        async fn without_token() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;
            let mut router = create_router(pool);
            let request = Request::builder().uri("/app").body(Body::empty())?;
            let response = router.call(request).await?;

            assert_eq!(response.status(), StatusCode::SEE_OTHER);

            Ok(())
        }

        #[tokio::test]
        async fn with_invalid_token() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;
            let mut router = create_router(pool);
            let request = Request::builder()
                .uri("/app")
                .header(COOKIE, "session=a")
                .body(Body::empty())?;
            let response = router.call(request).await?;

            assert_eq!(response.status(), StatusCode::SEE_OTHER);

            Ok(())
        }

        #[tokio::test]
        async fn with_valid_token() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;

            {
                let db = pool.get().await?;

                repo::auth::signup(&db, credentials::valid()).await?;
            };

            let mut router = create_router(pool);
            let request = Request::builder()
                .uri("/login")
                .method(Method::POST)
                .header(
                    CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.essence_str(),
                )
                .body(Body::new(
                    serde_urlencoded::to_string(credentials::valid())?,
                ))?;
            let response = router.call(request).await?;
            let cookie_header = response.headers().get(SET_COOKIE).unwrap();
            let request = Request::builder()
                .uri("/app")
                .header(COOKIE, cookie_header)
                .body(Body::empty())?;
            let response = router.call(request).await?;

            assert_eq!(response.status(), StatusCode::OK);

            Ok(())
        }
    }
}
