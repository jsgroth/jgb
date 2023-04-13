#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
// Remove pedantic lints that are very likely to produce false positives or that I disagree with
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::if_not_else)]
#![allow(clippy::inline_always)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::similar_names)]
#![allow(clippy::single_match_else)]
#![allow(clippy::stable_sort_primitive)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::unreadable_literal)]

mod app;

pub use app::{AppConfig, JgbApp};
