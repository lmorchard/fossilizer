#![warn(clippy::pedantic)]
// Curated allow-list: lints that are noise for a CLI app rather than a library.
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::unused_async,
    clippy::implicit_hasher,
    clippy::struct_excessive_bools,
    clippy::non_std_lazy_statics,
    clippy::unnecessary_debug_formatting
)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate tera;

pub mod activitystreams;
pub mod app;
pub mod config;
pub mod db;
pub mod downloader;
pub mod mastodon;
pub mod media;
pub mod site_generator;
pub mod templates;
pub mod themes;
pub mod util;
