mod app;
mod backend;
mod canvas;
mod cli;
mod gnome;
mod hands;
mod maker;
mod overlay;
mod render;
mod settings;
mod sound;
mod toolbar;
mod tray;

use gtk::glib::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--help" | "-h" | "help") => {
            print!("{}", cli::HELP);
            ExitCode::SUCCESS
        }
        Some("--version" | "-V") => {
            println!("tinyhand {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("list") => app::list(),
        Some("render") => render::run(&args[2..]),
        Some("make-hand") => maker::run(&args[2..]),
        Some("settings") => settings::run(),
        Some(command) if !cli::REMOTE.contains(&command) => {
            eprintln!("tinyhand: unknown command `{command}`. Try `tinyhand --help`.");
            ExitCode::FAILURE
        }
        _ => app::run(&args),
    }
}
