use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

use crate::messages::*;

pub struct Client {
    id: String,
    stream: TcpStream,
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
