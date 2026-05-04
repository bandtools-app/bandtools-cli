fn main() {
    if let Err(error) = bt::run() {
        eprintln!("bt: {error:#}");
        std::process::exit(1);
    }
}
