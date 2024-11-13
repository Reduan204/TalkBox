use std::sync::Arc;
use std::sync::Mutex;

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

#[tauri::command]
async fn create_server() -> Result<(), String> {
    // Rust's thread safe BS
    let users: Arc<Mutex<Vec<User>>> = Arc::new(Mutex::new(Vec::new()));

    let listener = TcpListener::bind("127.0.0.1:8080").await.map_err(|e| e.to_string())?;
    println!("Server created at 127.0.0.1:8080");

    // This is not japan's tokio that we are using
    tokio::spawn(async move {
        let users = Arc::clone(&users);
        while let Ok((mut socket, _)) = listener.accept().await {
            println!("Client connected: {:?}", socket.peer_addr());

            // We are creating a buffer to store some data inside of it
            let mut buffer = vec![0; 1024];
            let _ = socket.read(&mut buffer).await;

            let msg: ClientMessages = match serde_json::from_slice(&buffer) {
            Ok(msg) => msg,
            Err(_) => {
                    // Send error message if the incoming message is shit
                    let error_msg = ServerMessages::ServerError {
                        message: "Invalid message format".to_string(),
                    };
                    socket.write_all(&serde_json::to_vec(&error_msg).unwrap()).await.unwrap();
                    continue;
                }
            };

            // FIX: MSG not received

            match msg {
                ClientMessages::Connect { username } => {
                    // Create new user
                    let new_user = User {
                        username: username.clone(),
                    };

                    // Send confirmation message back to the client
                    let server_msg = ServerMessages::Connected {
                        username,
                    };
                    socket.write_all(&serde_json::to_vec(&server_msg).unwrap()).await.unwrap();

                    println!("New User: {}", new_user.username);

                    // Add new user to the server
                    let mut users_lock = users.lock().unwrap();
                    users_lock.push(new_user);
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

    // This will create new message to send to the server
    // The message is then serialized / turned into json file then convert it into bytes
    // TODO: Handle errors properly by not using unwrap, use expect or something
    let connect_message = ClientMessages::Connect { username: username.to_string() };
    let _ = stream.write_all(serde_json::to_string(&connect_message).unwrap().as_bytes()).await;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![join_server, create_server])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
