use crate::messages::*;

use tauri::Manager;
use tauri::State;
use tokio::sync::Mutex;
use tauri::Emitter;
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct User {
    id: String,
    username: String,
}

//static USERS: Lazy<Arc<Mutex<Vec<User>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));
//static MESSAGES: Lazy<Arc<Mutex<Vec<Message>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

#[tauri::command]
pub async fn create_server(
    ip: &str,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
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

pub async fn client_handler(
    mut socket: TcpStream,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    println!("Client connected: {:?}", socket.peer_addr());

    let user_state = app_handle.state::<Mutex<Vec<User>>>();
    let message_state = app_handle.state::<Mutex<Vec<Message>>>();

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
                    let mut users_lock = user_state.lock().await;
                    users_lock.push(new_user);
                }

                app_handle.emit("user_count_changed", get_users_len(user_state.clone()).await.unwrap()).unwrap();
                app_handle.emit("user_list_updated", get_users(user_state.clone()).await.unwrap()).unwrap();
            }

            ClientMessages::Message { id, content } => {
                let users = user_state.lock().await;
                if let Some(user) = users.iter().find(|u| u.id == id) {
                    let new_message = Message {
                        user_id: id.clone(),
                        username: user.username.clone(),
                        content,
                    };

                    // Store the message in the shared vector
                    let mut messages_lock = message_state.lock().await;
                    messages_lock.push(new_message);
                } else {
                    println!("Message received from unknown user ID: {}", id);
                }

                app_handle.emit("new_message", get_messages(message_state.clone()).await.unwrap()).unwrap();
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_messages(state: State<'_, Mutex<Vec<Message>>>) -> Result<Vec<Message>, String> {
    let messages_lock = state.lock().await;
    Ok(messages_lock.clone())
}

#[tauri::command]
pub async fn get_users(state: State<'_, Mutex<Vec<User>>>) -> Result<Vec<String>, String> {
    let users_lock = state.lock().await;
    let usernames: Vec<String> = users_lock.iter().map(|user| user.username.clone()).collect();
    Ok(usernames)
}

#[tauri::command]
pub async fn get_users_len(state: State<'_, Mutex<Vec<User>>>) -> Result<String, String> {
    let users_lock = state.lock().await;
    Ok(users_lock.len().to_string())
}
