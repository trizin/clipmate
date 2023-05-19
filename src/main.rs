use clap::{App, Arg, SubCommand};
use clipboard_manager_lib::manager::ClipboardManager;
use std::sync::Arc;
use std::{thread, time::Duration};

fn main() {
    let matches = App::new("clipmate")
        .version("0.1.0")
        .author("trizin")
        .about("Manages clipboard history")
        .subcommand(SubCommand::with_name("daemon").about("Starts the clipboard daemon"))
        .subcommand(SubCommand::with_name("history").about("Displays clipboard history"))
        .arg(
            Arg::with_name("item")
                .help("Item number to set to current clipboard")
                .index(1),
        )
        .get_matches();

    let history_file_path = Arc::new("./.clipboard_history.json".to_string());
    let mut manager = ClipboardManager::new(Arc::clone(&history_file_path));

    match matches.subcommand_name() {
        Some("daemon") => {
            thread::spawn(move || loop {
                manager.update_clipboard_content();
                manager.update_image_content();
                thread::sleep(Duration::from_millis(500));
            });
            loop {
                thread::sleep(Duration::from_millis(1000));
            }
        }
        Some("history") => {
            let history = manager.get_history();
            for (i, item) in history.items.iter().enumerate() {
                println!("{}: {} {:?}", i + 1, item.data, item.item_type);
            }
        }
        _ => {
            if let Some(item_number) = matches.value_of("item") {
                let item_number: usize = item_number
                    .parse()
                    .expect("Please provide a valid item number");
                manager.set_clipboard_text(item_number);
            }
        }
    }
}
