/**
 * Tivana Chrome Extension - Background Service Worker
 *
 * Manages WebSocket connection to Tivana runtime and forwards CDP commands
 * between the runtime and chrome.debugger API.
 */

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/** @type {WebSocket|null} */
let ws = null;

/** @type {boolean} */
let wsConnected = false;

/** Port number for Tivana runtime */
let port = 9876;

/** Session counter for generating IDs */
let sessionCounter = 0;

/**
 * Attached tabs: tabId → { targetId, sessionId, url, title }
 * @type {Map<number, {targetId: string, sessionId: string, url: string, title: string}>}
 */
const attachedTabs = new Map();

/** Locks to prevent double-attach races: Set of tabIds currently being attached/detached */
const operationLocks = new Set();

/** Reconnect state */
let reconnectDelay = 1000;
const MAX_RECONNECT_DELAY = 30000;
let reconnectTimer = null;

// ---------------------------------------------------------------------------
// Badge helpers
// ---------------------------------------------------------------------------

function setBadge(text, color) {
  chrome.action.setBadgeText({ text });
  chrome.action.setBadgeBackgroundColor({ color });
}

function updateBadge() {
  if (attachedTabs.size > 0 && wsConnected) {
    setBadge("ON", "#0ea5e9");
  } else if (attachedTabs.size > 0 && !wsConnected) {
    setBadge("!", "#ef4444");
  } else if (!wsConnected && reconnectTimer) {
    setBadge("\u2026", "#f59e0b");
  } else {
    setBadge("", "#888888");
  }
}

// ---------------------------------------------------------------------------
// WebSocket connection
// ---------------------------------------------------------------------------

function connectWebSocket() {
  if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) {
    return;
  }

  try {
    ws = new WebSocket(`ws://localhost:${port}`);
  } catch (e) {
    console.error("[tivana] WebSocket creation failed:", e);
    scheduleReconnect();
    return;
  }

  ws.onopen = async () => {
    console.log("[tivana] Connected to runtime");
    wsConnected = true;
    reconnectDelay = 1000;
    updateBadge();

    // Send handshake so runtime identifies this as an extension connection immediately
    wsSend({ method: "extension.hello", params: { version: "0.1.0" } });

    // Re-announce attached tabs — but verify debugger is still connected first
    for (const [tabId, info] of attachedTabs) {
      try {
        // Test if debugger is actually attached by sending a simple command
        await chrome.debugger.sendCommand({ tabId }, "Runtime.evaluate", {
          expression: "1",
          returnByValue: true,
        });
        // Debugger still works — re-announce
        wsSend({ method: "tab.attached", params: info });
      } catch (e) {
        console.log(`[tivana] Tab ${tabId} debugger stale, removing:`, e.message || e);
        attachedTabs.delete(tabId);
        // Try to re-attach
        try {
          await chrome.debugger.attach({ tabId }, "1.3");
          const tab = await chrome.tabs.get(tabId);
          const targets = await chrome.debugger.getTargets();
          const target = targets.find(t => t.tabId === tabId);
          info.targetId = target ? target.id : `tab-${tabId}`;
          info.url = tab.url || info.url;
          info.title = tab.title || info.title;
          attachedTabs.set(tabId, info);
          
          // Re-enable domains
          try { await chrome.debugger.sendCommand({ tabId }, "Runtime.enable"); } catch {}
          try { await chrome.debugger.sendCommand({ tabId }, "Page.enable"); } catch {}
          try { await chrome.debugger.sendCommand({ tabId }, "DOM.enable"); } catch {}
          
          wsSend({ method: "tab.attached", params: info });
          console.log(`[tivana] Tab ${tabId} re-attached successfully`);
        } catch (reattachErr) {
          console.log(`[tivana] Tab ${tabId} re-attach failed:`, reattachErr.message || reattachErr);
        }
      }
    }
    persistState();
  };

  ws.onmessage = (event) => {
    handleRuntimeMessage(event.data);
  };

  ws.onclose = () => {
    wsConnected = false;
    ws = null;
    updateBadge();
    scheduleReconnect();
  };

  ws.onerror = () => {
    // onclose will fire after this
  };
}

function scheduleReconnect() {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connectWebSocket();
  }, reconnectDelay);
  reconnectDelay = Math.min(reconnectDelay * 2, MAX_RECONNECT_DELAY);
  updateBadge();
}

function wsSend(msg) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg));
  }
}

// ---------------------------------------------------------------------------
// Handle messages from Tivana runtime
// ---------------------------------------------------------------------------

async function handleRuntimeMessage(data) {
  let msg;
  try {
    msg = JSON.parse(data);
  } catch {
    return;
  }

  // Ping/pong keepalive
  if (msg.method === "ping") {
    wsSend({ method: "pong" });
    return;
  }

  // Re-attach debugger to a tab (after navigation detaches it)
  if (msg.method === "tab.reattach") {
    const tabId = msg.params?.tabId;
    if (!tabId) {
      wsSend({ id: msg.id, error: "Missing tabId" });
      return;
    }
    try {
      // Get old session info if we had one
      const oldInfo = attachedTabs.get(tabId);
      const oldSessionId = oldInfo ? oldInfo.sessionId : `tivana-tab-${++sessionCounter}`;
      
      // Remove old entry if it exists
      attachedTabs.delete(tabId);
      
      // Re-attach
      await chrome.debugger.attach({ tabId }, "1.3");
      const tab = await chrome.tabs.get(tabId);
      const targets = await chrome.debugger.getTargets();
      const target = targets.find(t => t.tabId === tabId);
      const targetId = target ? target.id : `tab-${tabId}`;
      
      const info = {
        tabId,
        targetId,
        sessionId: oldSessionId,
        url: tab.url || "",
        title: tab.title || "",
      };
      attachedTabs.set(tabId, info);
      persistState();
      updateBadge();
      
      // Re-enable required CDP domains
      try { await chrome.debugger.sendCommand({ tabId }, "Runtime.enable"); } catch {}
      try { await chrome.debugger.sendCommand({ tabId }, "Page.enable"); } catch {}
      try { await chrome.debugger.sendCommand({ tabId }, "DOM.enable"); } catch {}
      
      // Notify runtime
      wsSend({
        method: "tab.attached",
        params: info,
      });
      
      wsSend({ id: msg.id, result: { reattached: true, ...info } });
    } catch (e) {
      wsSend({ id: msg.id, error: e.message || String(e) });
    }
    return;
  }

  // List attached tabs
  if (msg.method === "tabs.list") {
    const tabs = [];
    for (const [tabId, info] of attachedTabs) {
      tabs.push({ tabId, ...info });
    }
    wsSend({ id: msg.id, result: { tabs } });
    return;
  }

  // CDP command forwarding
  if (msg.method === "cdp" && msg.params) {
    handleCdpCommand(msg);
    return;
  }
}

async function handleCdpCommand(msg) {
  const { method, params, sessionId } = msg.params;

  // Find the tab for this sessionId
  let targetTabId = null;
  for (const [tabId, info] of attachedTabs) {
    if (info.sessionId === sessionId) {
      targetTabId = tabId;
      break;
    }
  }

  if (targetTabId === null) {
    wsSend({ id: msg.id, error: `No tab attached for session ${sessionId}` });
    return;
  }

  // Special case: Page.navigate — use chrome.tabs.update + re-attach debugger
  // because CDP navigation detaches the debugger in extension mode
  if (method === "Page.navigate" && params && params.url) {
    try {
      const oldInfo = attachedTabs.get(targetTabId);
      const oldSessionId = oldInfo ? oldInfo.sessionId : null;
      
      await chrome.tabs.update(targetTabId, { url: params.url });
      
      // Wait for the page to finish loading
      await new Promise((resolve) => {
        const onUpdated = (tabId, changeInfo) => {
          if (tabId === targetTabId && changeInfo.status === "complete") {
            chrome.tabs.onUpdated.removeListener(onUpdated);
            resolve();
          }
        };
        chrome.tabs.onUpdated.addListener(onUpdated);
        // Safety timeout — don't wait forever
        setTimeout(() => {
          chrome.tabs.onUpdated.removeListener(onUpdated);
          resolve();
        }, 15000);
      });
      
      // Re-attach debugger if it was detached during navigation
      if (!attachedTabs.has(targetTabId)) {
        try {
          await chrome.debugger.attach({ tabId: targetTabId }, "1.3");
          const tab = await chrome.tabs.get(targetTabId);
          const targets = await chrome.debugger.getTargets();
          const target = targets.find(t => t.tabId === targetTabId);
          const targetId = target ? target.id : `tab-${targetTabId}`;
          
          const info = {
            tabId: targetTabId,
            targetId,
            sessionId: oldSessionId || `tivana-tab-${++sessionCounter}`,
            url: tab.url || params.url,
            title: tab.title || "",
          };
          attachedTabs.set(targetTabId, info);
          persistState();
          updateBadge();
          
          // Re-enable required CDP domains
          try { await chrome.debugger.sendCommand({ tabId: targetTabId }, "Runtime.enable"); } catch {}
          try { await chrome.debugger.sendCommand({ tabId: targetTabId }, "Page.enable"); } catch {}
          try { await chrome.debugger.sendCommand({ tabId: targetTabId }, "DOM.enable"); } catch {}
          
          // Notify runtime of re-attachment
          wsSend({
            method: "tab.attached",
            params: info,
          });
        } catch (reattachErr) {
          // Debugger re-attach failed — notify but don't error the navigate
          console.warn("Failed to re-attach debugger after navigate:", reattachErr);
        }
      }
      
      wsSend({ id: msg.id, result: { frameId: "main", url: params.url } });
    } catch (e) {
      wsSend({ id: msg.id, error: e.message || String(e) });
    }
    return;
  }

  try {
    // Add a timeout to prevent hanging on navigation-triggering commands
    const result = await Promise.race([
      chrome.debugger.sendCommand({ tabId: targetTabId }, method, params || {}),
      new Promise((_, reject) => setTimeout(() => reject(new Error("CDP command timeout (25s)")), 25000)),
    ]);
    wsSend({ id: msg.id, result: result || {} });
  } catch (e) {
    wsSend({ id: msg.id, error: e.message || String(e) });
  }
}

// ---------------------------------------------------------------------------
// Chrome debugger attach / detach
// ---------------------------------------------------------------------------

async function attachTab(tabId) {
  if (attachedTabs.has(tabId) || operationLocks.has(tabId)) return;
  operationLocks.add(tabId);

  try {
    await chrome.debugger.attach({ tabId }, "1.3");

    const tab = await chrome.tabs.get(tabId);
    sessionCounter++;
    const sessionId = `tivana-tab-${sessionCounter}`;

    // Get the debugger target info
    const targets = await chrome.debugger.getTargets();
    const target = targets.find(t => t.tabId === tabId);
    const targetId = target ? target.id : `tab-${tabId}`;

    const info = {
      tabId,
      targetId,
      sessionId,
      url: tab.url || "",
      title: tab.title || "",
    };

    attachedTabs.set(tabId, info);
    persistState();
    updateBadge();

    // Enable CDP domains
    const domains = ["Page", "Runtime", "DOM", "Network", "Target"];
    for (const domain of domains) {
      try {
        await chrome.debugger.sendCommand({ tabId }, `${domain}.enable`, {});
      } catch {
        // Some domains may fail; that's ok
      }
    }

    // Notify Tivana
    wsSend({ method: "tab.attached", params: info });
  } catch (e) {
    console.error("[tivana] Failed to attach tab:", e.message || e);
    // Show error badge briefly so user knows something went wrong
    setBadge("!", "#ef4444");
    setTimeout(() => updateBadge(), 3000);
    // Send error to runtime for debugging
    wsSend({ method: "tab.error", params: { tabId, error: String(e.message || e) } });
  } finally {
    operationLocks.delete(tabId);
  }
}

async function detachTab(tabId, reason = "user") {
  if (!attachedTabs.has(tabId) || operationLocks.has(tabId)) return;
  operationLocks.add(tabId);

  const info = attachedTabs.get(tabId);

  try {
    await chrome.debugger.detach({ tabId });
  } catch {
    // Already detached
  }

  attachedTabs.delete(tabId);
  persistState();
  updateBadge();

  if (info) {
    wsSend({
      method: "tab.detached",
      params: {
        tabId,
        targetId: info.targetId,
        sessionId: info.sessionId,
        reason,
      },
    });
  }

  operationLocks.delete(tabId);
}

// ---------------------------------------------------------------------------
// Extension icon click: toggle attach/detach
// ---------------------------------------------------------------------------

chrome.action.onClicked.addListener(async (tab) => {
  if (!tab.id) return;

  if (attachedTabs.has(tab.id)) {
    await detachTab(tab.id, "user");
  } else {
    await attachTab(tab.id);
  }
});

// ---------------------------------------------------------------------------
// Chrome debugger events
// ---------------------------------------------------------------------------

chrome.debugger.onEvent.addListener((source, method, params) => {
  if (!source.tabId) return;
  const info = attachedTabs.get(source.tabId);
  if (!info) return;

  wsSend({
    method: "cdp.event",
    params: {
      method,
      params: params || {},
      sessionId: info.sessionId,
    },
  });
});

chrome.debugger.onDetach.addListener((source, reason) => {
  if (!source.tabId) return;
  const info = attachedTabs.get(source.tabId);
  if (!info) return;

  attachedTabs.delete(source.tabId);
  persistState();
  updateBadge();

  wsSend({
    method: "tab.detached",
    params: {
      tabId: source.tabId,
      targetId: info.targetId,
      sessionId: info.sessionId,
      reason: reason || "browser",
    },
  });
});

// ---------------------------------------------------------------------------
// Navigation tracking
// ---------------------------------------------------------------------------

chrome.webNavigation.onCommitted.addListener(async (details) => {
  if (details.frameId !== 0) return; // Only main frame
  const info = attachedTabs.get(details.tabId);
  if (!info) return;

  // Update stored info
  info.url = details.url;
  try {
    const tab = await chrome.tabs.get(details.tabId);
    info.title = tab.title || "";
  } catch {
    // Tab may be gone
  }

  persistState();

  wsSend({
    method: "tab.navigated",
    params: {
      tabId: details.tabId,
      targetId: info.targetId,
      sessionId: info.sessionId,
      url: info.url,
      title: info.title,
    },
  });

  // Re-enable CDP domains after navigation
  const domains = ["Page", "Runtime", "DOM", "Network", "Target"];
  for (const domain of domains) {
    try {
      await chrome.debugger.sendCommand({ tabId: details.tabId }, `${domain}.enable`, {});
    } catch {
      // Ignore
    }
  }
});

// ---------------------------------------------------------------------------
// Keepalive via chrome.alarms
// ---------------------------------------------------------------------------

chrome.alarms.create("tivana-keepalive", { periodInMinutes: 25 / 60 });

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "tivana-keepalive") {
    // Poke the WebSocket to keep the service worker alive
    if (ws && ws.readyState === WebSocket.OPEN) {
      wsSend({ method: "pong" });
    }
  }
});

// ---------------------------------------------------------------------------
// Persistence: save/restore state across service worker restarts
// ---------------------------------------------------------------------------

function persistState() {
  const tabs = {};
  for (const [tabId, info] of attachedTabs) {
    tabs[tabId] = info;
  }
  chrome.storage.local.set({ tivanaState: { tabs, sessionCounter, port } });
}

async function restoreState() {
  try {
    const data = await chrome.storage.local.get("tivanaState");
    if (data.tivanaState) {
      const state = data.tivanaState;
      if (state.port) port = state.port;
      if (state.sessionCounter) sessionCounter = state.sessionCounter;

      // Restore attached tabs - verify they're still attached
      if (state.tabs) {
        for (const [tabIdStr, info] of Object.entries(state.tabs)) {
          const tabId = parseInt(tabIdStr, 10);
          try {
            // Check if tab still exists
            await chrome.tabs.get(tabId);
            // Check if debugger is still attached
            const targets = await chrome.debugger.getTargets();
            const isAttached = targets.some(t => t.tabId === tabId && t.attached);
            if (isAttached) {
              attachedTabs.set(tabId, info);
            }
          } catch {
            // Tab gone, skip
          }
        }
      }
    }
  } catch (e) {
    console.error("[tivana] Failed to restore state:", e);
  }
}

// ---------------------------------------------------------------------------
// Settings: load port from options
// ---------------------------------------------------------------------------

async function loadSettings() {
  try {
    const data = await chrome.storage.local.get("tivanaSettings");
    if (data.tivanaSettings && data.tivanaSettings.port) {
      port = data.tivanaSettings.port;
    }
  } catch {
    // Use default
  }
}

// Listen for settings changes
chrome.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes.tivanaSettings) {
    const newSettings = changes.tivanaSettings.newValue;
    if (newSettings && newSettings.port && newSettings.port !== port) {
      port = newSettings.port;
      // Reconnect with new port
      if (ws) {
        ws.close();
      }
      connectWebSocket();
    }
  }
});

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

async function init() {
  await loadSettings();
  await restoreState();
  updateBadge();
  connectWebSocket();
}

init();
