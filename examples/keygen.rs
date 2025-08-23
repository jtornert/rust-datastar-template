use axum_extra::extract::cookie::Key;
use base64::{Engine, prelude::BASE64_URL_SAFE};

fn main() {
    println!("{}", BASE64_URL_SAFE.encode(Key::generate().master()));
}
