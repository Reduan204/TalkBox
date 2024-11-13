import { invoke } from "@tauri-apps/api/core";

function toggleJoinPanel() {
  const joinPanel = document.getElementById("join-panel");
  const mainElement = document.querySelector("main");

  // Toggle the visibility of join-panel
  if (joinPanel.style.display === "flex") {
    joinPanel.style.display = "none";
    mainElement.style.filter = "none";
  } else { // TODO: Maybe some blur?
    joinPanel.style.display = "flex";
    //mainElement.style.filter = "blur(40px)";
  }
}

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

async function createServer() {
  invoke("create_server")
    .then(() => console.log("Server created"))
    .catch((error) => console.error(error));
}

async function joinServer() {
  const usernameInput = document.getElementById("username-input");
  const serverIPInput = document.getElementById("server-ip-input");

  const username = usernameInput.value;
  const ip = serverIPInput.value;

  invoke("join_server", { ip, username })
    .then(() => {
      console.log("Joined server")
      updateUserCount();
      displayUsers();
    })
    .catch((error) => console.error(error));
}

window.addEventListener("DOMContentLoaded", () => {
  updateUserCount();
  displayUsers();

  const createServerButton = document.querySelector("#create-server");
  createServerButton?.addEventListener("click", () => {
    createServer();
  });

  const joinServerButton = document.querySelector("#join-server");
  joinServerButton?.addEventListener("click", () => {
    toggleJoinPanel();
  });

  const connectButton = document.getElementById("connect-btn");
  connectButton.addEventListener("click", () => {
    joinServer();
    displayUsers();
    toggleJoinPanel();
  });
});

