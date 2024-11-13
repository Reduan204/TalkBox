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
    .then(() => console.log("Joined server"))
    .catch((error) => console.error(error));
}

window.addEventListener("DOMContentLoaded", () => {
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
    toggleJoinPanel();
  });
});
