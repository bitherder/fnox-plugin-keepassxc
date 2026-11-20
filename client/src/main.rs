fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to start the async runtime");
    std::process::exit(runtime.block_on(kpxc_client::cli::run(&args)));
}
