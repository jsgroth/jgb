#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, rust_2018_idioms)]
// Remove pedantic lints that are very likely to produce false positives or that I disagree with
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::if_not_else,
    clippy::inline_always,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::single_match_else,
    clippy::stable_sort_primitive,
    clippy::struct_excessive_bools,
    clippy::unreadable_literal
)]

mod app;

pub use app::{AppConfig, JgbApp};
