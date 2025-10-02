pub mod home {
    use std::{convert::Infallible, sync::Arc, time::Duration};

    use asynk_strim::{Yielder, stream_fn};
    use axum::{
        Extension,
        response::{Html, IntoResponse, Response, Sse, sse::Event},
    };
    use datastar::{consts::ElementPatchMode, prelude::PatchElements};
    use serde::Serialize;
    use tera_template_macro::TeraTemplate;
    use uuid::Uuid;

    use crate::{
        models::user::User,
        repo::{self, Db, DbRecord},
        web::{
            DEFAULT_LOCALE, TEMPLATES,
            guards::DatastarRequest,
            pages::{NotFound, ServiceUnavailable},
        },
    };

    #[derive(Serialize, TeraTemplate)]
    #[template(path = "pages/app/home.j2")]
    struct Page {
        user: DbRecord<User>,
    }

    pub async fn get(
        Extension(db): Extension<Arc<Db<'_>>>,
        DatastarRequest(datastar_request): DatastarRequest,
    ) -> Response {
        if datastar_request {
            Sse::new(stream_fn(
                |mut yielder: Yielder<Result<Event, Infallible>>| async move {
                    for _ in 0..3 {
                        yielder
                            .yield_item(Ok(PatchElements::new("<p>Hello</p>")
                                .mode(ElementPatchMode::Append)
                                .selector("#root")
                                .write_as_axum_sse_event()))
                            .await;
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                },
            ))
            .into_response()
        } else {
            let user = match repo::user::me(&db).await {
                Err(repo::Error::NotFound) => {
                    return Html(NotFound {}.render(TEMPLATES.read().await, DEFAULT_LOCALE))
                        .into_response();
                }
                Err(repo::Error::ServiceUnavailable(uuid)) => {
                    return Html(
                        ServiceUnavailable { uuid }.render(TEMPLATES.read().await, DEFAULT_LOCALE),
                    )
                    .into_response();
                }
                Err(_) => {
                    let uuid = Uuid::new_v4();
                    crate::log_line!(uuid);
                    return Html(
                        ServiceUnavailable { uuid }.render(TEMPLATES.read().await, DEFAULT_LOCALE),
                    )
                    .into_response();
                }
                Ok(user) => user,
            };

            Html(Page { user }.render(TEMPLATES.read().await, DEFAULT_LOCALE)).into_response()
        }
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
        use surrealdb::RecordId;
        use tower::Service;

        use crate::{
            _test::{TestResult, create_test_pool, credentials, setup_test},
            models::user::User,
            repo::{self, DbRecord},
            web::{DEFAULT_LOCALE, TEMPLATES, create_router, pages::app::home::Page},
        };

        #[tokio::test]
        async fn page() -> TestResult {
            setup_test();

            Page {
                user: DbRecord {
                    id: RecordId::from_table_key("user", 1),
                    data: User {
                        name: "username".into(),
                    },
                },
            }
            .render(TEMPLATES.read().await, DEFAULT_LOCALE);

            Ok(())
        }

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
