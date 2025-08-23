use serde::Serialize;

pub mod index {
    use axum::response::{Html, IntoResponse};
    use serde::Serialize;
    use tera_template_macro::TeraTemplate;

    use crate::web::TEMPLATES;

    #[derive(Serialize, TeraTemplate)]
    #[template(path = "pages/index.j2")]
    struct Page {}

    pub async fn get() -> impl IntoResponse {
        Html(Page {}.render(TEMPLATES.read().await, "en-GB"))
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
            web,
        };

        #[tokio::test]
        async fn get() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;
            let mut router = web::create_router(pool);
            let request = Request::builder().uri("/").body(Body::empty())?;
            let response = router.call(request).await?;

            assert_eq!(response.status(), StatusCode::OK);

            Ok(())
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum AuthType {
    SignUp,
    LogIn,
}

pub mod signup {
    use std::sync::Arc;

    use axum::{
        Extension,
        response::{Html, IntoResponse, Redirect, Response},
    };
    use axum_extra::extract::Form;
    use serde::Serialize;
    use tera_template_macro::TeraTemplate;

    use crate::{
        repo::{self, Db, auth::Credentials},
        web::{TEMPLATES, pages::public::AuthType},
    };

    #[derive(Serialize, TeraTemplate)]
    #[template(path = "pages/signup.j2")]
    struct Page {
        errors: Vec<String>,
        auth_type: AuthType,
    }

    pub async fn get() -> impl IntoResponse {
        Html(
            Page {
                errors: Vec::new(),
                auth_type: AuthType::SignUp,
            }
            .render(TEMPLATES.read().await, "en-GB"),
        )
    }

    pub async fn post(
        Extension(db): Extension<Arc<Db<'_>>>,
        Form(body): Form<Credentials>,
    ) -> Result<impl IntoResponse, Response> {
        match repo::auth::signup(&db, body).await {
            Err(repo::Error::UsernameTaken(username)) => Err(Html(
                Page {
                    errors: vec![format!("username {username} already taken")],
                    auth_type: AuthType::SignUp,
                }
                .render(TEMPLATES.read().await, "en-GB"),
            )
            .into_response()),

            Err(repo::Error::InvalidUsername) => Err(Html(
                Page {
                    errors: vec![format!("username invalid")],
                    auth_type: AuthType::SignUp,
                }
                .render(TEMPLATES.read().await, "en-GB"),
            )
            .into_response()),

            Err(_) => Err(Html(
                Page {
                    errors: vec!["something went wrong".into()],
                    auth_type: AuthType::SignUp,
                }
                .render(TEMPLATES.read().await, "en-GB"),
            )
            .into_response()),

            _ => Ok(Redirect::to("/login")),
        }
    }

    #[cfg(test)]
    mod tests {
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode, header::CONTENT_TYPE},
        };
        use tower::Service;

        use crate::{
            _test::{TestResult, create_test_pool, credentials, setup_test},
            web,
        };

        #[tokio::test]
        async fn get() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;
            let mut router = web::create_router(pool);
            let request = Request::builder().uri("/signup").body(Body::empty())?;
            let response = router.call(request).await?;

            assert_eq!(response.status(), StatusCode::OK);

            Ok(())
        }

        #[tokio::test]
        async fn with_valid_credentials() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;
            let mut router = web::create_router(pool);
            let request = Request::builder()
                .uri("/signup")
                .method(Method::POST)
                .header(
                    CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.essence_str(),
                )
                .body(serde_urlencoded::to_string(credentials::valid())?)?;
            let response = router.call(request).await?;

            assert_eq!(
                response.status(),
                StatusCode::SEE_OTHER,
                "{:?}",
                axum::body::to_bytes(response.into_body(), usize::MAX).await?
            );

            Ok(())
        }

        #[tokio::test]
        async fn with_invalid_credentials() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;
            let mut router = web::create_router(pool);
            let request = Request::builder()
                .uri("/signup")
                .method(Method::POST)
                .header(
                    CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.essence_str(),
                )
                .body(serde_urlencoded::to_string(credentials::invalid())?)?;
            let response = router.call(request).await?;

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{:?}",
                axum::body::to_bytes(response.into_body(), usize::MAX).await?
            );

            Ok(())
        }
    }
}

pub mod login {
    use std::sync::Arc;

    use axum::{
        Extension,
        response::{Html, IntoResponse, Redirect, Response},
    };
    use axum_extra::extract::{
        Form, PrivateCookieJar,
        cookie::{Cookie, SameSite},
    };
    use serde::Serialize;
    use tera_template_macro::TeraTemplate;

    use crate::{
        repo::{self, Db, auth::Credentials},
        web::{SESSION_COOKIE_NAME, TEMPLATES, pages::public::AuthType},
    };

    #[derive(Serialize, TeraTemplate)]
    #[template(path = "pages/login.j2")]
    struct Page {
        errors: Vec<String>,
        auth_type: AuthType,
    }

    pub async fn get() -> impl IntoResponse {
        Html(
            Page {
                errors: Vec::new(),
                auth_type: AuthType::LogIn,
            }
            .render(TEMPLATES.read().await, "en-GB"),
        )
    }

    pub async fn post(
        secrets: PrivateCookieJar,
        Extension(db): Extension<Arc<Db<'_>>>,
        Form(body): Form<Credentials>,
    ) -> Result<impl IntoResponse, Response> {
        let token = match repo::auth::login(&db, body).await {
            Err(repo::error::Error::CredentialsInvalid) => {
                return Err(Html(
                    Page {
                        errors: vec!["credentials invalid".into()],
                        auth_type: AuthType::LogIn,
                    }
                    .render(TEMPLATES.read().await, "en-GB"),
                )
                .into_response());
            }
            Err(e) => {
                tracing::error!(?e);
                return Err(Html(
                    Page {
                        errors: vec!["something went wrong".into()],
                        auth_type: AuthType::LogIn,
                    }
                    .render(TEMPLATES.read().await, "en-GB"),
                )
                .into_response());
            }
            Ok(token) => token,
        };

        Ok((
            secrets.add(
                Cookie::build((SESSION_COOKIE_NAME, token))
                    .http_only(true)
                    .secure(true)
                    .same_site(SameSite::Strict),
            ),
            Redirect::to("/"),
        ))
    }

    #[cfg(test)]
    mod tests {
        use axum::{
            body::Body,
            http::{Method, Request, StatusCode, header::CONTENT_TYPE},
        };
        use tower::Service;

        use crate::{
            _test::{TestResult, create_test_pool, credentials, setup_test},
            repo, web,
        };

        #[tokio::test]
        async fn get() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;
            let mut router = web::create_router(pool);
            let request = Request::builder().uri("/login").body(Body::empty())?;
            let response = router.call(request).await?;

            assert_eq!(response.status(), StatusCode::OK);

            Ok(())
        }

        #[tokio::test]
        async fn with_valid_user() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;

            {
                let db = pool.get().await?;

                repo::auth::signup(&db, credentials::valid()).await?;
            }

            let mut router = web::create_router(pool);
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

            assert_eq!(
                response.status(),
                StatusCode::SEE_OTHER,
                "{:?}",
                axum::body::to_bytes(response.into_body(), usize::MAX).await?
            );

            Ok(())
        }

        #[tokio::test]
        async fn with_invalid_user() -> TestResult {
            setup_test();

            let pool = create_test_pool().await?;

            {
                let db = pool.get().await?;

                repo::auth::signup(&db, credentials::valid()).await?;
            }

            let mut router = web::create_router(pool);
            let request = Request::builder()
                .uri("/login")
                .method(Method::POST)
                .header(
                    CONTENT_TYPE,
                    mime::APPLICATION_WWW_FORM_URLENCODED.essence_str(),
                )
                .body(Body::new(serde_urlencoded::to_string(
                    credentials::invalid(),
                )?))?;
            let response = router.call(request).await?;

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{:?}",
                axum::body::to_bytes(response.into_body(), usize::MAX).await?
            );

            Ok(())
        }
    }
}
