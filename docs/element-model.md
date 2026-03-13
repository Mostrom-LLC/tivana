# Element Model

How page elements are represented — the full visual and semantic model that lets agents see what humans see.


---

# Complete Element Schema
```typescript
interface Element {
  // Identity
  id: string;
  
  // Semantic
  role: string;           // button, textbox, link, heading
  label: string;          // Accessible name
  value?: string;         // Current value
  text?: string;          // Visible text
  
  // State
  focused: boolean;
  enabled: boolean;
  visible: boolean;
  interactable: boolean;
  
  // Geometry
  bounds: { x, y, width, height };
  padding: { top, right, bottom, left };
  margin: { top, right, bottom, left };
  
  // Typography
  font: { family, size, weight, color, lineHeight };
  textAlign: string;
  
  // Colors
  background: string;
  
  // Borders
  border: { width, style, color, radius };
  
  // Layout
  display: string;
  flexDirection?: string;
  justifyContent?: string;
  alignItems?: string;
  
  // Visual State
  opacity: number;
  cursor: string;
  overflow: string;
  
  // Accessibility
  contrastRatio?: number;
  focusVisible?: boolean;
  tabIndex?: number;
  ariaAttributes?: Record<string, string>;
  headingLevel?: number;
  altText?: string;
  
  // Hierarchy
  children?: Element[];
}
```

---

# Property Categories
## Semantic (what it is)
- role — Accessibility role (button, textbox, link, image, heading)
- label — Accessible name (what screen readers announce)
- value — Current value for form elements
- text — Visible text content
## Visual (what it looks like)
- font — family, size, weight, color, lineHeight
- background — Background color
- border — width, style, color, radius
- opacity — Transparency (0-1)
## Geometry (where it is)
- bounds — Position and size (x, y, width, height)
- padding — Internal spacing
- margin — External spacing
## Accessibility (compliance data)
- contrastRatio — Computed text/background contrast
- focusVisible — Has visible focus indicator
- tabIndex — Keyboard navigation order
- ariaAttributes — All ARIA properties

---

# Data Sources
- Accessibility tree (AXTree) — Roles, labels, states
- Computed styles (getComputedStyle) — Colors, fonts, borders
- Layout engine (getBoundingClientRect) — Position, size
- IntersectionObserver — Visibility in viewport
