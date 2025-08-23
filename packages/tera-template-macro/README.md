# Tera Template Macro

## Overview

This crate provides a proc macro for the [tera-hot-reload](https://github.com/oxidlabs/tera-hot-reload) crate, allowing for easier and more efficient templating in Rust.

## Usuage

Basic usuage example

```rust
use tera_hot_reload::TeraTemplate;

// create static tera
pub static TERA: LazyLock<RwLock<Tera>> = LazyLock::new(|| {
    RwLock::new(tera::Tera::new("templates/**/*").expect("Failed to create Tera instance"))
});

// Create a template
#[derive(TeraTemplate)]
#[template(path="index.html")]
struct HelloTemplate {
    name: String,
    greeting: String,
}

//                                  axum::response::IntoResponse
async fn index() -> impl IntoResponse {
    let context = HelloTemplate {
        name: "World".to_string(),
        greeting: "Hello".to_string()
    };
    
    // axum::response::Html
    Html(context.render(&TERA.read().unwrap()))
}
```

## Dependencies

The following dependencies are required to use this crate:

*   [Tera](https://keats.github.io/tera/docs/) (version 1.20.0 or higher)
*   [syn](https://docs.rs/syn/latest/syn/) (version 2.0.72 or higher)
*   [quote](https://docs.rs/quote/latest/quote/) (version 1.0.37 or higher)
*   [proc-macro2](https://docs.rs/proc-macro2/1.0.86/proc_macro2/) (version 1.0.86 or higher)

## Contributing

If you want to contribute to this project, please feel free to create a new branch and submit a pull request. You can also check out the [Tera documentation](https://docs.rs/tera/1.20.0/terra/index.html) for more information on how to use the Tera template engine.

## License

This crate is released under the MIT license. Please see the `LICENSE` file for more details.
