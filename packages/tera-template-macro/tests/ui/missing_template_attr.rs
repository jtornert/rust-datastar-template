use tera_template_macro::TeraTemplate;

#[derive(TeraTemplate, serde::Serialize)]
struct Smoke {
    hello: String
}

fn main() {}