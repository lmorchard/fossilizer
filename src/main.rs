mod cli;

#[macro_use]
extern crate log;

fn main() {
    match cli::execute() {
        Ok(_) => {}
        Err(err) => println!("Error: {:?}", err),
    }
}
