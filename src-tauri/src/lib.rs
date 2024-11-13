use std::sync::Arc;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessages {
    Connected { username: String },
    ServerError { message: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessages {
    Connect { username: String },
    Message { content: String },
}

#[derive(Clone, Debug)]
struct User {
    username: String,
}

// Rust's thread safe BS
static USERS: Lazy<Arc<Mutex<Vec<User>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

#[tauri::command]
async fn create_server() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:8080").await.map_err(|e| e.to_string())?;
    println!("Server created at 127.0.0.1:8080");

    // This is not japan's tokio that we are using
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            println!("Client connected: {:?}", socket.peer_addr());

            // ----------
            let mut buffer = vec![0; 1024];
            let bytes_read = match socket.read(&mut buffer).await {
                Ok(size) if size > 0 => size,
                Ok(_) => continue,
                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                    continue;
                }
            };

            // ----------
            let msg = match serde_json::from_slice::<ClientMessages>(&buffer[..bytes_read]) {
                Ok(msg) => msg,
                Err(_) => {
                    let error_msg = ServerMessages::ServerError {
                        message: "Invalid message format".to_string(),
                    };
                    let _ = socket.write_all(&serde_json::to_vec(&error_msg).unwrap()).await;
                    continue;
                }
            };

            // ----------
            match msg {
                ClientMessages::Connect { username } => {
                    let new_user = User {
                        username: username.clone(),
                    };

                    // Send confirmation message back to client
                    let server_msg = ServerMessages::Connected { username: username.clone() };
                    if socket.write_all(&serde_json::to_vec(&server_msg).unwrap()).await.is_ok() {
                        println!("New user connected: {}", username);

                        // Add new user to the server user list
                        let mut users_lock = USERS.lock().unwrap();
                        users_lock.push(new_user);
                    }
                }
                ClientMessages::Message { content: _ } => {} // TODO: Send message content's
                _ => {}
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn join_server(ip: &str, username: &str) -> Result<(), String> {
    let mut stream = TcpStream::connect(ip).await.map_err(|e| e.to_string())?;
    println!("Connected to the server at {ip}");

    // Create the connect message
    let connect_message = ClientMessages::Connect { username: username.to_string() };

    // Serialize the message to JSON and write it to the stream
    let message_json = serde_json::to_string(&connect_message).map_err(|e| e.to_string())?;
    stream.write_all(message_json.as_bytes()).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_users() -> Result<Vec<String>, String> {
    let users_lock = USERS.lock().map_err(|e| e.to_string())?;
    let usernames: Vec<String> = users_lock.iter().map(|user| user.username.clone()).collect();
    Ok(usernames)
}

#[tauri::command]
async fn get_users_len() -> Result<String, String> {
    let users_lock = USERS.lock().map_err(|e| e.to_string())?;
    Ok(users_lock.len().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![join_server, create_server, get_users, get_users_len])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
