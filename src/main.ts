import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';

type Message = {
  username: string;
  content: string;
};

// ----------------------------------------
async function togglePanel(panelName: string) {
  const panel = document.getElementById(panelName);

  // Check if the panel exists before toggeling
  if (!panel) {
    console.error(`Panel with ID '${panelName}' not found.`);
    return;
  }

  // Toggle the visibility of the panel
  if (panel.style.display === "flex") {
    panel.style.display = "none";
  } else {
    panel.style.display = "flex";
  }
}

// ----------------------------------------
async function displayUsers() {
  try {
    const usernames: string[] = await invoke("get_users");
    const usersContainer = document.getElementById("users-container");
    
    if (usersContainer) {
      usersContainer.innerHTML = "";
      usernames.forEach((username) => {
        const userElement = document.createElement("p");
        userElement.textContent = username;
        usersContainer.appendChild(userElement);
      });
    }
  } catch (error) {
    console.error("Failed to fetch users:", error);
  }
}

// ----------------------------------------
async function updateUserCount() {
  try {
    const userCount: string = await invoke("get_users_len");
    const totalUsersElement = document.getElementById("total-users");
    if (totalUsersElement) {
      totalUsersElement.textContent = `Total Users: ${userCount}`;
    }
  } catch (error) {
    console.error("Failed to fetch user count:", error);
  }
}

// ----------------------------------------
async function sendMessage() {
  const messageInput = document.getElementById("message-maker") as HTMLInputElement;

  if (messageInput) {
    const content = messageInput.value.trim();

    if (content) {
      try {
        await invoke("send_message", { content });
        console.log("Message sent:", content);
        messageInput.value = "";
      } catch (error) {
        console.error("Failed to send message:", error);
      }
    }
  }
}

async function loadMessages() {
  try {
    const messages: Message[] = await invoke("get_messages");
    const messagesContainer = document.getElementById("chat");

    if (messagesContainer) {
      messagesContainer.innerHTML = '';

      messages.forEach((message: any) => {
        const messageElement = document.createElement("div");
        messageElement.classList.add("message");
        messageElement.innerHTML = `
          <div class="profile"></div>
          <hr>
          <h5>${message.username}</h5>
          <p>${message.content}</p>
        `;
        messagesContainer.appendChild(messageElement);
      });
    }
  } catch (error) {
    console.error("Failed to load messages:", error);
  }
}

// ----------------------------------------
async function createServer() {
  const serverIPInput = document.getElementById("s-server-ip-input") as HTMLInputElement;

  const ip = serverIPInput.value;

  invoke("create_server", { ip })
    .then(() => console.log("Server created"))
    .catch((error) => console.error(error));
}

// ----------------------------------------
async function joinServer() {
  const usernameInput = document.getElementById("j-username-input") as HTMLInputElement;
  const serverIPInput = document.getElementById("j-server-ip-input") as HTMLInputElement;

  const username = usernameInput.value;
  const ip = serverIPInput.value;

  invoke("join_server", { ip, username })
    .then(() => console.log("Joined server"))
    .catch((error) => console.error(error));
}

listen("user_count_changed", () => { updateUserCount(); });
listen("user_list_updated", () => { displayUsers(); });
listen("new_message", () => { loadMessages(); });

window.addEventListener("DOMContentLoaded", () => {
  const serverPanelButton = document.querySelector("#create-server");
  serverPanelButton?.addEventListener("click", () => {
    togglePanel("server-panel");
  });

  const createServerButton = document.querySelector("#j-create-server");
  createServerButton?.addEventListener("click", () => {
    createServer();
    togglePanel("server-panel");
  });

  const joinServerButton = document.querySelector("#join-server");
  joinServerButton?.addEventListener("click", () => {
    togglePanel("join-panel");
  });

  const connectButton = document.getElementById("j-connect-btn") as HTMLButtonElement;
  connectButton.addEventListener("click", () => {
    joinServer();
    togglePanel("join-panel");
  });

  document.getElementById("message-maker")?.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      sendMessage();
    }
  });
});

// Global variables
(window as any).togglePanel = togglePanel;
