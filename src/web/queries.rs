use serde::Deserialize;

#[derive(Deserialize)]
pub(in crate::web) struct RedirectToQuery {
    pub redirect_to: Option<String>,
}
