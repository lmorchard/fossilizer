mod cli;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() {
    match cli::execute().await {
        Ok(_) => {}
        Err(err) => println!("Error: {err:?}"),
    }
}
