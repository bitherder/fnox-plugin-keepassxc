fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some(reference) => match kpxc_client::parse_reference(reference) {
            Some(name) => {
                eprintln!("not yet implemented: would look up entries tagged fnox://{name}");
                std::process::exit(1);
            }
            None => {
                eprintln!("invalid reference: {reference} (expected fnox://<name>)");
                std::process::exit(2);
            }
        },
        None => {
            eprintln!("usage: kpxc-client <fnox://name>");
            std::process::exit(2);
        }
    }
}
