fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some(reference) => match kpxc_client::parse_reference(reference) {
            Some(name) => {
                // TODO: replace with a real KeePassXC Browser Protocol lookup.
                // For now this just echoes the parsed name, which is enough
                // to prove the fnox Lua plugin -> this binary -> stdout path
                // end to end (see spec/integration).
                println!("{name}");
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
