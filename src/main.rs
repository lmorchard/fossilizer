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

mod cli;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    if let Err(err) = cli::execute().await {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
