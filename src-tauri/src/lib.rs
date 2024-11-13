use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessages {
    Connected { id: String },
    ServerError { message: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessages {
    Connect { username: String },
    Message { id: String, content: String },
}

#[derive(Clone, Debug)]
struct User {
    id: String,
    username: String,
}

struct Client {
    id: String,
    stream: TcpStream,
}

// Rust's thread safe BS
// Use HashMap instead ?
static USERS: Lazy<Arc<Mutex<Vec<User>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

#[tauri::command]
async fn create_server() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .map_err(|e| e.to_string())?;
    println!("Server created at 127.0.0.1:8080");

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
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
                    }

                    ClientMessages::Message { id, content } => {
                        let users = USERS.lock().await;
                        if let Some(user) = users.iter().find(|u| u.id == id) {
                            println!("{}: {}", user.username, content);
                        } else {
                            println!("Message received from unknown user ID: {}", id);
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

impl Client {
    pub async fn new(ip: &str, username: &str) -> Result<Self, String> {
        let mut stream = TcpStream::connect(ip).await.map_err(|e| e.to_string())?;
        println!("Connected to the server at {ip}");

        let connect_message = ClientMessages::Connect { username: username.to_string() };
        let message_json = serde_json::to_string(&connect_message).map_err(|e| e.to_string())?;
        stream.write_all(message_json.as_bytes()).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;

        // Buffer to read the server response
        let mut buffer = vec![0; 1024];
        let n = stream.read(&mut buffer).await.map_err(|e| e.to_string())?;

        // Deserialize the response
        let server_msg: ServerMessages = serde_json::from_slice(&buffer[..n]).map_err(|e| e.to_string())?;

        // Extract the UUID and log it for verification
        let user_id = if let ServerMessages::Connected { id } = server_msg {
            id
        } else {
            return Err("Failed to receive user ID from server".to_string());
        };

        Ok(Self {
            id: user_id,
            stream,
        })
    }

    pub async fn send_message(&mut self, content: String) -> Result<(), String> {
        let message = ClientMessages::Message {
            id: self.id.clone(),
            content,
        };

        // Serialize the message to JSON
        let message_json = serde_json::to_string(&message).map_err(|e| format!("Serialization error: {}", e))?;

        // Try sending the message over the stream
        let write_result = self.stream.write_all(message_json.as_bytes()).await;

        if let Err(e) = write_result {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Err("Connection closed. Please reconnect.".to_string());
            } else {
                return Err(format!("Write error: {}", e));
            }
        }

        // Flush the stream
        self.stream.flush().await.map_err(|e| format!("Flush error: {}", e))?;

        Ok(())
    }
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
        .manage(Mutex::new(None::<Client>)) // TODO: Manage Server same way as the client
        .invoke_handler(tauri::generate_handler![
            join_server,
            create_server,
            send_message,
            get_users,
            get_users_len,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
