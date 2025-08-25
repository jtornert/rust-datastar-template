// #![deny(clippy::cargo)]
#![deny(clippy::complexity)]
#![deny(clippy::correctness)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![deny(clippy::perf)]
#![deny(clippy::blanket_clippy_restriction_lints)]
#![deny(clippy::style)]
#![deny(clippy::suspicious)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
// #![cfg_attr(debug_assertions, warn(clippy::cargo))]
#![cfg_attr(debug_assertions, warn(clippy::complexity))]
#![cfg_attr(debug_assertions, warn(clippy::correctness))]
#![cfg_attr(debug_assertions, warn(clippy::nursery))]
#![cfg_attr(debug_assertions, warn(clippy::pedantic))]
#![cfg_attr(debug_assertions, warn(clippy::perf))]
#![cfg_attr(debug_assertions, warn(clippy::blanket_clippy_restriction_lints))]
#![cfg_attr(debug_assertions, warn(clippy::style))]
#![cfg_attr(debug_assertions, warn(clippy::suspicious))]
#![cfg_attr(debug_assertions, warn(clippy::unwrap_used))]
#![cfg_attr(debug_assertions, warn(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]

#[cfg(debug_assertions)]
mod _dev;
#[cfg(test)]
mod _test;
mod config;
mod error;
mod models;
mod repo;
mod web;

use config::CONFIG;
use error::{Error, Result};
pub use web::start;
