use std::convert::Infallible;

use axum::{extract::FromRequestParts, http::HeaderMap};

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
        let headers = HeaderMap::from_request_parts(parts, state).await?;
        let is_datastar_request = headers
            .get("datastar-request")
            .is_some_and(|h| h.as_bytes() == b"true");

        Ok(Self(is_datastar_request))
    }
}
