pub mod app;
pub mod public;

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use serde::Serialize;
use tera_template_macro::TeraTemplate;

use crate::web::TEMPLATES;

#[derive(Serialize, TeraTemplate)]
#[template(path = "pages/not_found.j2")]
struct NotFound {}

pub async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(NotFound {}.render(TEMPLATES.read().await, "en-GB")),
    )
}

#[derive(Serialize, TeraTemplate)]
#[template(path = "pages/service_unavailable.j2")]
pub struct ServiceUnavailable {}
