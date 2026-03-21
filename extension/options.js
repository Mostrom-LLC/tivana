/**
 * Tivana Extension - Options page
 */

const portInput = document.getElementById("port");
const saveBtn = document.getElementById("save");
const savedMsg = document.getElementById("saved");
const statusDot = document.getElementById("statusDot");
const statusText = document.getElementById("statusText");
const tabList = document.getElementById("tabList");

// Load saved settings
chrome.storage.local.get("tivanaSettings", (data) => {
  if (data.tivanaSettings && data.tivanaSettings.port) {
    portInput.value = data.tivanaSettings.port;
  }
});

// Save settings
saveBtn.addEventListener("click", () => {
  const port = parseInt(portInput.value, 10);
  if (port < 1 || port > 65535 || isNaN(port)) return;

  chrome.storage.local.set({ tivanaSettings: { port } }, () => {
    savedMsg.style.opacity = "1";
    setTimeout(() => { savedMsg.style.opacity = "0"; }, 1500);
  });
});

// Poll status
function updateStatus() {
  chrome.storage.local.get("tivanaState", (data) => {
    const state = data.tivanaState || {};
    const tabs = state.tabs || {};
    const tabEntries = Object.entries(tabs);

    // Check connection by attempting to read state
    // We infer connected if there are attached tabs or the background is running
    const hasAttached = tabEntries.length > 0;

    // Try a simple fetch to see if runtime is up
    const port = parseInt(portInput.value, 10) || 9876;
    fetch(`http://localhost:${port}`, { mode: "no-cors" })
      .then(() => {
        statusDot.className = "dot connected";
        statusText.textContent = "Connected";
      })
      .catch(() => {
        statusDot.className = "dot disconnected";
        statusText.textContent = "Disconnected";
      });

    // Update tab list
    if (tabEntries.length === 0) {
      tabList.innerHTML = '<li class="empty">No tabs attached</li>';
    } else {
      tabList.innerHTML = tabEntries
        .map(([, info]) =>
          `<li>
            <span class="tab-url" title="${info.url}">${info.title || info.url}</span>
            <span class="tab-session">${info.sessionId}</span>
          </li>`
        )
        .join("");
    }
  });
}

updateStatus();
setInterval(updateStatus, 3000);
