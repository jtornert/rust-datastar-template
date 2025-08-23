use tera_template_macro::TeraTemplate;

#[derive(TeraTemplate, serde::Serialize)]
#[template(path = 69)]
struct Smoke {
    hello: String
}

fn main() {}