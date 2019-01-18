mod cargo;
mod cli;

fn main() {
    if let Err(err) = cli::run() {
        println!("error: {}", err);
        std::process::exit(1);
    }
}
