use tera_template_macro::TeraTemplate;

#[derive(TeraTemplate, serde::Serialize)]
#[template(not_path = "hello world")]
struct Smoke {
    hello: String
}

fn main() {}