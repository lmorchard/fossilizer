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
