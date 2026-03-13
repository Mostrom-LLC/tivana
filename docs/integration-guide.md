# Integration Guide

How to connect an AI agent to Tivana.


---

# Quick Start
## 1. Install
```bash
npm install tivana
```
## 2. Start the runtime
```bash
npx tivana
```
This launches a Chromium browser and starts the WebSocket server.

## 3. Connect your agent
```typescript
import { observe, act } from "tivana";

observe((page) => {
  console.log(`URL: ${page.url}`);
  console.log(`Elements: ${page.elements.length}`);
});

await act.navigate("https://github.com");
await act.click({ role: "link", label: "Sign in" });
```

---

# Working with Page State
```typescript
observe((page) => {
  // Find all buttons
  const buttons = page.elements.filter(e => e.role === "button");
  
  // Find element by label
  const submitBtn = page.elements.find(
    e => e.role === "button" && e.label === "Submit"
  );
  
  // Check visibility
  const visibleInputs = page.elements.filter(
    e => e.role === "textbox" && e.visible
  );
  
  // Check accessibility
  const lowContrast = page.elements.filter(
    e => e.contrastRatio && e.contrastRatio < 4.5
  );
});
```

---

# Performing Actions
```typescript
// Click by element ID
await act.click("e42");

// Click by semantic selector
await act.click({ role: "button", label: "Sign in" });

// Type into focused element
await act.type("hello world");

// Type into specific element
await act.type("user@example.com", { target: "email-input" });

// Navigate
await act.navigate("https://example.com");

// Scroll element into view
await act.scroll("footer");
```

---

# Full Example: Login Flow
```typescript
import { observe, act, waitFor } from "tivana";

async function login(username: string, password: string) {
  await act.navigate("https://app.example.com/login");
  
  await waitFor((page) => 
    page.elements.some(e => e.role === "textbox" && e.label === "Username")
  );
  
  await act.click({ role: "textbox", label: "Username" });
  await act.type(username);
  
  await act.click({ role: "textbox", label: "Password" });
  await act.type(password);
  
  await act.click({ role: "button", label: "Sign in" });
  
  await waitFor((page) => page.url.includes("/dashboard"));
  
  console.log("Login successful!");
}
```

---

# Integrating with LLMs
```typescript
import { observe, act } from "tivana";
import Anthropic from "@anthropic-ai/sdk";

const goal = "Find the pricing page";

observe(async (page) => {
  const response = await anthropic.messages.create({
    model: "claude-sonnet-4-20250514",
    messages: [{
      role: "user",
      content: `Goal: ${goal}\n\nPage: ${page.url}\nElements: ${JSON.stringify(page.elements.slice(0, 50))}\n\nWhat action? Respond with JSON: { action, target }`
    }]
  });
  
  const action = JSON.parse(response.content[0].text);
  await act[action.action](action.target);
});
```
