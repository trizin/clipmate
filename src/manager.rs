use clipboard::{ClipboardContext, ClipboardProvider};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClipboardItemType {
    TEXT,
    IMAGE,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClipboardItem {
    pub time: u128,
    pub item_type: ClipboardItemType,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ClipboardHistory {
    pub items: Vec<ClipboardItem>,
    image_counter: u128,
    text_counter: u128,
}

impl ClipboardHistory {
    fn new() -> ClipboardHistory {
        ClipboardHistory {
            items: Vec::new(),
            image_counter: 0,
            text_counter: 0,
        }
    }

    fn add_item(&mut self, item: String, item_type: ClipboardItemType) {
        if item_type == ClipboardItemType::IMAGE {
            self.image_counter += 1;
        }
        if item_type == ClipboardItemType::TEXT {
            self.text_counter += 1;
        }
        self.items.push(ClipboardItem {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u128,
            item_type: item_type,
            data: item,
        });
    }

    fn get_item(&self, index: usize) -> Option<&ClipboardItem> {
        self.items.get(index)
    }

    fn get_counter(&self, item_type: ClipboardItemType) -> u128 {
        match item_type {
            ClipboardItemType::TEXT => self.text_counter,
            ClipboardItemType::IMAGE => self.image_counter,
        }
    }
}

#[derive(Default)]
pub struct ClipboardManager {
    history: ClipboardHistory,
    history_file_path: Arc<String>,
    xclip_available: bool,
}

impl ClipboardManager {
    pub fn new(history_file_path: Arc<String>) -> ClipboardManager {
        let history: ClipboardHistory;
        let file = File::open(&*history_file_path);

        match file {
            Ok(mut f) => {
                let mut contents = String::new();
                f.read_to_string(&mut contents)
                    .expect("Failed to read the file");
                history = serde_json::from_str(&contents).expect("Failed to parse the file");
            }
            Err(_) => {
                history = ClipboardHistory::new();
            }
        }

        ClipboardManager {
            history,
            history_file_path,
            xclip_available: true,
        }
    }

    pub fn save_text(&mut self, text: String) {
        if text.len() < 1 {
            return;
        }
        self.history.add_item(text, ClipboardItemType::TEXT);
        self.save_history();
    }

    fn save_history(&self) {
        let history_string = serde_json::to_string(&self.history).unwrap();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&*self.history_file_path)
            .unwrap();
        file.write_all(history_string.as_bytes())
            .expect("Failed to write to the file");
    }

    pub fn get_history(&self) -> &ClipboardHistory {
        &self.history
    }

    pub fn set_clipboard_text(&self, item_number: usize) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        match self.history.get_item(item_number - 1) {
            Some(item) => {
                match item.item_type {
                    ClipboardItemType::TEXT => {
                        ctx.set_contents(item.data.to_string()).unwrap();
                    }
                    ClipboardItemType::IMAGE => {
                        Command::new("xclip")
                            .args(&["-selection", "clipboard", "-t", "image/png"])
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .unwrap()
                            .stdin
                            .unwrap()
                            .write_all(
                                std::fs::read(&item.data)
                                    .expect("Failed to read image file")
                                    .as_slice(),
                            )
                            .unwrap();
                    }
                }
                println!("Clipboard set to item {}", item_number);
            }
            None => {
                println!("Item {} not found in clipboard history", item_number);
            }
        }
    }

    pub fn save_image(&mut self, image_data: Vec<u8>, image_name: String) {
        if std::path::Path::new(&image_name).exists() {
            return;
        }

        let mut file = File::create(&image_name).expect("Failed to create image file");
        file.write_all(&image_data)
            .expect("Failed to write to image file");

        self.history
            .add_item(image_name.clone(), ClipboardItemType::IMAGE);
        self.save_history();
        // println!("Image saved as {}", image_name);
    }

    fn last_clipboard_image(&self) -> String {
        self.history
            .items
            .iter()
            .rev()
            .find(|x| x.item_type == ClipboardItemType::IMAGE)
            .map_or_else(|| String::from(""), |item| item.data.clone())
    }

    fn last_clipboard_text(&self) -> String {
        self.history
            .items
            .iter()
            .rev() // reverse the iterator so we start from the end
            .find(|x| x.item_type == ClipboardItemType::TEXT)
            .map_or_else(|| String::from(""), |item| item.data.clone())
    }

    pub fn update_clipboard_content(&mut self) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        match ctx.get_contents() {
            Ok(content) => {
                if content != self.last_clipboard_text() {
                    self.save_text(content);
                }
            }
            Err(_) => {
                println!("Error while getting text content from the clipboard");
            }
        }
    }

    pub fn update_image_content(&mut self) {
        if !self.xclip_available {
            return;
        }
        let output = Command::new("xclip")
            .args(&["-selection", "clipboard", "-t", "image/png", "-o"])
            .output();

        if output.is_err() {
            self.xclip_available = false;
            return;
        }

        let image_data = output.unwrap().stdout;

        if !image_data.is_empty() && image_data.len() > 50 {
            // hash image_data
            let mut hasher = Sha256::new();
            hasher.update(&image_data);
            let result = hasher.finalize();
            let image_name = format!("{:x}.png", result);

            if self
                .history
                .items
                .iter()
                .find(|x| x.data == image_name)
                .is_some()
            {
                return;
            }

            self.save_image(image_data, image_name);
        }
    }
}
