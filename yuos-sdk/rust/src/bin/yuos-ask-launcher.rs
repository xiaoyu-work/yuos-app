fn main() {
    if let Err(error) = yuos_sdk::ask_yu::run_sdk_launcher() {
        eprintln!("Ask Yu OS launcher failed: {error}");
        std::process::exit(1);
    }
}
