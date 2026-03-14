# Element Model

How page elements are represented — the semantic and visual model that lets agents understand what humans see.

---

## Element Schema

Elements returned by `perceive.elements` have this structure:

```typescript
interface Element {
  // === Identity ===
  id: string;              // Stable ID (e.g., "e1", "e2")

  // === Semantic ===
  role: string;            // Accessibility role (button, textbox, link, etc.)
  name?: string;           // Accessible name/label
  value?: string;          // Current value (inputs, selects)
  description?: string;    // Accessible description

  // === Geometry ===
  bounds?: BoundingBox;    // Position and size

  // === Styles ===
  styles?: ElementStyles;  // Computed visual styles

  // === State Flags ===
  focused: boolean;        // Has keyboard focus
  enabled: boolean;        // Is enabled (not disabled)
  checked?: boolean;       // Checkbox/radio state
  selected?: boolean;      // Option selection state
  expanded?: boolean;      // Accordion/dropdown expanded
  required?: boolean;      // Form field required

  // === Hierarchy ===
  children?: Element[];    // Child elements (if any)
}
```

---

## Bounding Box

Position and size of the element:

```typescript
interface BoundingBox {
  x: number;       // Left position (px from viewport left)
  y: number;       // Top position (px from viewport top)
  width: number;   // Width in pixels
  height: number;  // Height in pixels
}
```

**Note:** Bounds are relative to the viewport, not the document. Use scroll position from `PageState` to calculate document coordinates.

---

## Element Styles

Subset of computed CSS styles:

```typescript
interface ElementStyles {
  fontFamily?: string;      // e.g., "Inter, sans-serif"
  fontSize?: string;        // e.g., "16px"
  fontWeight?: string;      // e.g., "400", "bold"
  color?: string;           // Text color (CSS value)
  backgroundColor?: string; // Background color (CSS value)
  border?: string;          // Border shorthand
  display?: string;         // e.g., "block", "flex", "none"
  visibility?: string;      // e.g., "visible", "hidden"
}
```

---

## Element Roles

Common roles returned by Tivana:

| Role | Description | Typical elements |
|------|-------------|------------------|
| `button` | Clickable button | `<button>`, `[role="button"]` |
| `link` | Navigable link | `<a href>`, `[role="link"]` |
| `textbox` | Text input | `<input type="text">`, `<textarea>` |
| `checkbox` | Checkbox | `<input type="checkbox">` |
| `radio` | Radio button | `<input type="radio">` |
| `combobox` | Dropdown select | `<select>`, autocomplete |
| `menuitem` | Menu item | `[role="menuitem"]` |
| `tab` | Tab control | `[role="tab"]` |
| `switch` | Toggle switch | `[role="switch"]` |
| `slider` | Range slider | `<input type="range">` |
| `searchbox` | Search input | `<input type="search">` |

---

## Element ID Stability

Element IDs (`e1`, `e2`, etc.) follow these rules:

1. **Session-stable**: IDs persist within a page session
2. **Re-assigned on navigation**: New page = new IDs
3. **May change on major DOM mutation**: Large DOM changes may invalidate IDs

### Stale ID Behavior

If an action targets a stale element ID:
- Runtime returns `target_not_found` error
- Agent should re-perceive to get fresh IDs

---

## Targeting Elements

Actions can target elements multiple ways:

### By Element ID (Primary)

Use the `id` from `perceive.elements`:

```typescript
client.click("e5");  // Click element with ID "e5"
```

### By CSS Selector

Use standard CSS selectors:

```typescript
client.click("button.submit");
```

### By Role and Label

Use semantic matching:

```typescript
client.click({ role: "button", label: "Submit" });
```

### By Coordinates

Direct click at coordinates:

```typescript
// Via ActionTarget
{ coordinates: [100, 200] }
```

---

## Interactive Element Detection

Tivana finds interactive elements using these selectors:

- `a[href]` — Links
- `button` — Buttons
- `input`, `select`, `textarea` — Form elements
- `[role="button"]`, `[role="link"]`, etc. — ARIA roles
- `[tabindex]:not([tabindex="-1"])` — Keyboard-focusable
- `[contenteditable="true"]` — Editable content

Hidden elements (`display: none`, `visibility: hidden`, zero-size) are excluded.

---

## Example Element

A login button might look like:

```json
{
  "id": "e3",
  "role": "button",
  "name": "Sign in",
  "bounds": {
    "x": 320,
    "y": 450,
    "width": 120,
    "height": 44
  },
  "styles": {
    "fontFamily": "Inter, sans-serif",
    "fontSize": "16px",
    "fontWeight": "600",
    "color": "rgb(255, 255, 255)",
    "backgroundColor": "rgb(59, 130, 246)",
    "border": "none",
    "display": "flex"
  },
  "focused": false,
  "enabled": true
}
```

---

## Data Sources

Element data is collected from:

| Property | Source |
|----------|--------|
| `role`, `name`, `value` | Accessibility tree (ARIA) |
| `bounds` | `getBoundingClientRect()` |
| `styles` | `getComputedStyle()` |
| `focused` | `document.activeElement` |
| `enabled` | Element `disabled` property |
| `checked`, `selected` | Element state properties |

---

## Design Notes

### No Screenshots

Tivana provides **semantic perception**, not screenshots. Elements include enough visual data (bounds, styles) for agents to understand layout without pixel data.

### Compact Output

Element data is normalized and compact:
- Only interactive elements by default
- Optional fields omitted when null/empty
- Styles are a subset (not all 300+ CSS properties)

### AI-Friendly

Elements are designed for AI consumption:
- Stable IDs for action targeting
- Semantic roles for understanding purpose
- Visual styles for understanding appearance
