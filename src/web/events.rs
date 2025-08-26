use serde::Serialize;
use tera_template_macro::TeraTemplate;

#[derive(Serialize, TeraTemplate)]
#[template(path = "events/redirect.js")]
pub(in crate::web) struct Redirect {
    pub href: String,
}
