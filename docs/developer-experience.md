# Developer Experience

# 3-Step Setup
### Step 1: Install
```bash
npm install tivana
```
### Step 2: Start
```bash
npx tivana
```
→ Browser opens. Ready for agent perception.

### Step 3: Perceive
```typescript
import { observe } from 'tivana';

const page = observe();
```

---

# What the Agent Sees
```typescript
{
  url: "https://github.com/login",
  title: "Sign in to GitHub",
  
  elements: [
    { id: "e1", role: "textbox", label: "Username", focused: true, bounds: {x: 200, y: 150, w: 280, h: 40} },
    { id: "e2", role: "textbox", label: "Password", bounds: {x: 200, y: 210, w: 280, h: 40} },
    { id: "e3", role: "button", label: "Sign in", enabled: true, bounds: {x: 200, y: 280, w: 280, h: 40} },
    { id: "e4", role: "link", label: "Forgot password?", bounds: {x: 200, y: 340, w: 120, h: 20} }
  ],
  
  focusedElement: "e1",
  scrollPosition: { x: 0, y: 0 },
  viewport: { width: 1280, height: 720 },
  timestamp: 1710354315000
}
```
Semantic. Compact. Everything an agent needs to understand the page.


---

# How the Agent Acts
```typescript
import { act } from 'tivana';

// By element ID
await act.click("e3");

// By role + label
await act.click({ role: "button", label: "Sign in" });

// Type into focused element
await act.type("myusername");

// Navigate
await act.navigate("https://github.com");
```
Actions reference elements semantically — not by coordinates, not by CSS selectors.


---

# Streaming Perception
The agent doesn't poll. It receives a continuous stream.

```typescript
import { observe } from 'tivana';

observe((page) => {
  // Called whenever page state changes
  // Agent can reason and react in real-time
  
  console.log(`Now at: ${page.url}`);
  console.log(`Elements: ${page.elements.length}`);
  console.log(`Focused: ${page.focusedElement}`);
});
```
## Mutation Awareness
```typescript
observe((page, mutations) => {
  // mutations[] tells the agent what just changed
  // - Element added
  // - Element removed  
  // - Element property changed
  // - Focus moved
  // - Navigation occurred
  
  for (const m of mutations) {
    if (m.type === 'added' && m.element.role === 'dialog') {
      // A modal just appeared — agent notices
    }
  }
});
```

---

# Why This Matters
### Playwright/Puppeteer
- Execute predefined scripts
- Blind between steps
- Check assertions, miss everything else
### Screenshots + Vision
- Heavy (send full image every time)
- Lossy (OCR errors, layout confusion)
- Point-in-time (no streaming)
- Can't reference elements directly
### This Protocol
- Semantic (roles, labels, not pixels)
- Streaming (continuous awareness)
- Lightweight (~5KB/update vs ~500KB/screenshot)
- Referenceable (element IDs for actions)
> 💡 Humans catch bugs tests miss because we see the whole page and notice when something feels off. This gives agents the same awareness.

---

# The Perception Loop
```plain text
Browser ──► Stream ──► Agent
   │                    │
   │  PageState         │ Reasoning
   │  - elements[]      │ - What do I see?
   │  - mutations[]     │ - What should I do?
   │                    ▼
   ◄───────────────── Action
       click / type / navigate
```
The agent perceives, reasons, acts. Continuously. Like a human.


---

# Summary
Install. Start. Perceive.

```bash
npm install tivana
npx tivana
```
```typescript
import { observe, act } from "tivana";
```
The agent now has eyes.


---

# Full Visual Awareness
The agent sees everything a human sees — not just semantic structure, but the full visual presentation.

```typescript
{
  id: "e3",
  role: "button",
  label: "Sign in",
  text: "Sign in",
  
  // Position & Size
  bounds: { x: 200, y: 280, width: 280, height: 40 },
  
  // Typography
  font: {
    family: "Inter, sans-serif",
    size: "16px",
    weight: 600,
    color: "#ffffff"
  },
  
  // Background & Borders
  background: "#238636",
  border: {
    width: "1px",
    style: "solid",
    color: "#238636",
    radius: "6px"
  },
  
  // Spacing
  padding: { top: 8, right: 16, bottom: 8, left: 16 },
  margin: { top: 16, right: 0, bottom: 0, left: 0 },
  
  // Alignment
  textAlign: "center",
  display: "flex",
  justifyContent: "center",
  alignItems: "center",
  
  // Visual State
  opacity: 1,
  visible: true,
  enabled: true,
  cursor: "pointer"
}
```
## What Humans Notice
- Colors — Is the button the right shade of green? Does text have enough contrast?
- Typography — Is the font readable? Is the weight consistent? Is text truncated?
- Alignment — Are elements lined up? Is spacing consistent?
- Borders — Are corners rounded correctly? Is the border visible?
- Spacing — Is there enough padding? Are margins consistent?
- States — Does hover look right? Is disabled state obvious?
> 💡 This is why Playwright tests pass but humans catch bugs. Tests check if the button exists — humans notice the button is the wrong color or misaligned by 2 pixels.
## Complete Element Model
Every element includes:

- Semantic — role, label, value, focused, enabled, interactable
- Geometry — bounds (x, y, width, height), padding, margin
- Typography — font family, size, weight, color, line-height, text-align
- Colors — background, foreground, border colors
- Borders — width, style, color, radius
- Layout — display, flex properties, alignment
- Visual State — opacity, visibility, overflow, cursor
- Content — text content, placeholder, alt text
Sourced from the browser computed styles — what the user actually sees rendered, not what the CSS says.


---

# Accessibility Awareness
The agent has everything it needs to understand accessibility issues and ADA/WCAG compliance.

## What the Agent Can Detect
- Color contrast — Has foreground + background colors, can calculate WCAG contrast ratio
- Missing labels — Knows if interactive elements lack accessible names
- Focus visibility — Can see focus styles (or lack thereof)
- Text size — Knows if text is too small to read
- Touch targets — Has element bounds, can verify 44x44px minimum
- Semantic structure — Has roles, can verify proper heading hierarchy
- Image alt text — Knows if images have descriptions
- Keyboard navigation — Can track focus order through the page
## Example: Agent Audits a Page
```typescript
// Agent receives this element:
{
  id: "e5",
  role: "button",
  label: "",  // ❌ Missing accessible name
  text: "",
  bounds: { width: 32, height: 32 },  // ❌ Below 44x44 touch target
  font: { color: "#999999" },
  background: "#f0f0f0",  // ❌ Contrast ratio 2.8:1 (needs 4.5:1)
  focus: { outline: "none" }  // ❌ No visible focus indicator
}

// Agent reasons:
// "This button has 4 accessibility issues:
//  1. No accessible name - screen readers cant announce it
//  2. Touch target 32x32px is below 44x44px minimum
//  3. Color contrast 2.8:1 fails WCAG AA (needs 4.5:1)
//  4. No focus outline - keyboard users cant see focus"
```
> ♿ The agent doesnt just detect issues — it understands WHY they are issues because it has the same visual information a human accessibility auditor would have.
## WCAG Data Points
Each element includes accessibility-relevant computed values:

- contrastRatio — Computed ratio between text color and background
- focusVisible — Whether element has visible focus styles
- tabIndex — Keyboard navigation order
- ariaAttributes — All ARIA properties (expanded, pressed, etc.)
- headingLevel — For heading elements (h1-h6)
- altText — For images
