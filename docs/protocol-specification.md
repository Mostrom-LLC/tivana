# Protocol Specification

Message formats for communication between runtime and agent.


---

# Transport
- WebSocket connection between runtime and agent
- JSON message format
- Bidirectional: runtime pushes state, agent sends actions

---

# Runtime → Agent Messages
## page.state
Full page state snapshot. Sent on navigation and periodically.

```typescript
{
  type: "page.state",
  url: string,
  title: string,
  elements: Element[],
  focusedElement: string | null,
  scrollPosition: { x: number, y: number },
  viewport: { width: number, height: number },
  timestamp: number
}
```
## page.mutation
Incremental change to page state.

```typescript
{
  type: "page.mutation",
  mutations: [
    { type: "added", element: Element },
    { type: "removed", elementId: string },
    { type: "changed", elementId: string, changes: Partial<Element> }
  ],
  timestamp: number
}
```
## action.result
Result of an agent action.

```typescript
{
  type: "action.result",
  actionId: string,
  success: boolean,
  error?: string
}
```

---

# Agent → Runtime Messages
## action.click
```typescript
{
  type: "action.click",
  actionId: string,
  target: string | { role: string, label: string },
  button?: "left" | "right" | "middle",
  clickCount?: 1 | 2
}
```
## action.type
```typescript
{
  type: "action.type",
  actionId: string,
  text: string,
  target?: string  // element ID, or focused element if omitted
}
```
## action.navigate
```typescript
{
  type: "action.navigate",
  actionId: string,
  url: string
}
```
## action.scroll
```typescript
{
  type: "action.scroll",
  actionId: string,
  target: string,  // element ID to scroll into view
  behavior?: "smooth" | "instant"
}
```
