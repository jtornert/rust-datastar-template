use tera_template_macro::TeraTemplate;

#[derive(TeraTemplate, serde::Serialize)]
#[template(path = "index69.html")]
struct Smoke {
    hello: String
}

fn main() {}