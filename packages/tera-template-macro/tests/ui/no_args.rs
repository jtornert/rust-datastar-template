use tera_template_macro::TeraTemplate;

#[derive(TeraTemplate, serde::Serialize)]
#[template]
struct Smoke {
    hello: String
}

fn main() {}