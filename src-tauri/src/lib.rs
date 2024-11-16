mod client;
mod server;
mod messages;

use crate::client::*;
use crate::server::*;

use messages::Message;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(RwLock::new(None::<Client>))
        .manage(Mutex::new(Vec::<User>::new()))
        .manage(Mutex::new(Vec::<Message>::new()))
        .invoke_handler(tauri::generate_handler![
            join_server,
            create_server,
            send_message,
            get_messages,
            get_users,
            get_users_len,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
