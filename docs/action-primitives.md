# Action Primitives

Available actions that agents can perform through the protocol.


---

## Input Actions
### click
Click an element by ID. Supports left, right, and middle click. Can specify single or double click.

### type
Type text into the currently focused element. Supports key modifiers (Ctrl, Shift, etc.) and special keys (Enter, Tab, Escape).

### hover
Move mouse over an element to trigger hover states, tooltips, and dropdown menus.

### drag
Drag from one element to another. Used for drag-and-drop interfaces, sliders, and resizing.


---

## Navigation Actions
### navigate
Navigate to a URL. Can be absolute or relative.

### scroll
Scroll an element into view, or scroll by a specified amount. Supports smooth and instant scrolling.

### back / forward
Navigate browser history backward or forward.


---

## Wait Actions
### waitForElement
Wait until an element with specified properties appears on the page.

### waitForNavigation
Wait until page navigation completes (useful after clicking links).

