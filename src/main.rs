fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let file_to_open = if args.len() == 2 {
        Some(args[1].clone())
    } else {
        None
    };

    ui::run(file_to_open);
}
