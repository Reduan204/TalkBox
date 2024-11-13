mod messages;
mod client;

use std::sync::Arc;

use crate::messages::*;
use crate::client::*;

use once_cell::sync::Lazy;
use tauri::Emitter;
use tauri::State;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use uuid::Uuid;

// TODO: broadcast all users
// BUG: Second client doesnt receive anything (Can connect tho)

// TODO: Replace User with Client
#[derive(Clone, Debug)]
struct User {
    id: String,
    username: String,
}

// Rust's thread safe BS
// Use HashMap instead ?
// FIX: Why tf this is still an user and not Client
static USERS: Lazy<Arc<Mutex<Vec<User>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));
static MESSAGES: Lazy<Arc<Mutex<Vec<Message>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

#[tauri::command]
async fn create_server(ip: &str, app_handle: tauri::AppHandle) -> Result<(), String> {
    let listener = TcpListener::bind(ip.to_string())
        .await
        .map_err(|e| e.to_string())?;
    println!("Server created at {}", ip.to_string());

    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            tokio::spawn(client_handler(socket, app_handle.clone()));
        }
    });

    Ok(())
}

async fn client_handler(mut socket: TcpStream, app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("Client connected: {:?}", socket.peer_addr());

    // Keep the connection alive
    loop {
        let mut buffer = vec![0; 1024];
        let bytes_read = match socket.read(&mut buffer).await {
            Ok(size) if size > 0 => size,
            Ok(_) => {
                println!("Client disconnected");
                break;
            }
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                break;
            }
        };

        // Deserialize the incoming message
        let msg = match serde_json::from_slice::<ClientMessages>(&buffer[..bytes_read]) {
            Ok(msg) => msg,
            Err(_) => {
                let error_msg = ServerMessages::ServerError {
                    message: "Invalid message format".to_string(),
                };
                if socket.write_all(&serde_json::to_vec(&error_msg).unwrap()).await.is_ok() {
                    println!("Sent error: Invalid message format");
                }
                continue; // Continue listening for new messages
            }
        };

        // Handle different message types
        match msg {
            ClientMessages::Connect { username } => {
                let id = Uuid::new_v4().to_string();
                let new_user = User {
                    id: id.clone(),
                    username: username.clone(),
                };

                // Send confirmation message back to client
                let server_msg = ServerMessages::Connected { id: id.clone() };
                if socket.write_all(&serde_json::to_vec(&server_msg).unwrap()).await.is_ok() {
                    println!("New user connected:\n| ID: {}\n| Name: {}", id, username);

                    // Add new user to the server user list
                    let mut users_lock = USERS.lock().await;
                    users_lock.push(new_user);
                }

                app_handle.emit("user_count_changed", get_users_len().await.unwrap()).unwrap();
                app_handle.emit("user_list_updated", get_users().await.unwrap()).unwrap();
            }

            ClientMessages::Message { id, content } => {
                let users = USERS.lock().await;
                if let Some(user) = users.iter().find(|u| u.id == id) {
                    let new_message = Message {
                        user_id: id.clone(),
                        username: user.username.clone(),
                        content,
                    };

                    // Store the message in the shared vector
                    let mut messages_lock = MESSAGES.lock().await;
                    messages_lock.push(new_message);
                } else {
                    println!("Message received from unknown user ID: {}", id);
                }

                app_handle.emit("new_message", get_messages().await.unwrap()).unwrap();
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn join_server(ip: &str, username: &str, state: State<'_, Mutex<Option<Client>>>) -> Result<(), String> {
    let mut client_lock = state.lock().await;

    if client_lock.is_none() {
        let client = Client::new(ip, username).await?;
        *client_lock = Some(client);
    }

    Ok(())
}

#[tauri::command]
async fn send_message(content: String, state: State<'_, Mutex<Option<Client>>>) -> Result<(), String> {
    // Lock the state and check if there is a client
    let mut client_lock = state.lock().await;
    
    if let Some(client) = client_lock.as_mut() {
        // If the client exists, proceed to send the message
        client.send_message(content).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        // If the client doesn't exist, return an error
        Err("Client is not connected".to_string())
    }
}

#[tauri::command]
async fn get_messages() -> Result<Vec<Message>, String> {
    let messages_lock = MESSAGES.lock().await;
    Ok(messages_lock.clone())
}

#[tauri::command]
async fn get_users() -> Result<Vec<String>, String> {
    let users_lock = USERS.lock().await;
    let usernames: Vec<String> = users_lock.iter().map(|user| user.username.clone()).collect();
    Ok(usernames)
}

#[tauri::command]
async fn get_users_len() -> Result<String, String> {
    let users_lock = USERS.lock().await;
    Ok(users_lock.len().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(None::<Client>))
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
