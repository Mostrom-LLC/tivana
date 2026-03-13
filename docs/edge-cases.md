# Edge Cases

Challenging scenarios the protocol must handle gracefully.


---

# Dynamic Content
## Infinite scroll
New elements load as user scrolls. Agent must handle growing element list and maintain stable references for existing elements.

## Live updates
Chat apps, dashboards, feeds that update without user action. Agent receives mutation stream but must not get overwhelmed by high-frequency updates.

## Lazy loading
Images and content that load when scrolled into view. Element may exist but content not yet rendered.

## SPAs and client-side routing
Page content changes without navigation event. Must detect soft navigations and rebuild element tree.


---

# Complex UI Patterns
## Modals and overlays
Content appears on top of other content. Agent must understand z-index and which elements are actually interactive.

## Dropdowns and menus
Elements that appear on hover or click. Agent may need to trigger hover state to see menu contents.

## Canvas and WebGL
Rendered graphics without DOM elements. Limited to bounding box — no internal structure visible.

## iframes
Embedded content from other origins. May have limited or no access depending on CORS. Cross-origin iframes are opaque.

## Shadow DOM
Web components with encapsulated DOM. Must pierce shadow roots to see internal structure.


---

# Visual Complexity
## CSS transforms and animations
Elements may be rotated, scaled, or moving. Bounds must reflect transformed position.

## Pseudo-elements
::before and ::after content not in DOM but visually present. May contain icons or decorations.

## Background images with text
Text rendered in images, not as DOM text. Not accessible to protocol — would need vision model.

## Overlapping elements
Multiple elements at same position. Must determine which is on top and actually clickable.


---

# Authentication & Security
## CAPTCHAs
Intentionally blocks automation. No good solution — may require human intervention or CAPTCHA solving service.

## Bot detection
Sites that detect automation via fingerprinting, behavior analysis. May need stealth mode or realistic input timing.

## OAuth popups
Authentication flows that open new windows. Must track and interact with popup windows.


---

# Performance & Scale
## Large DOM (10,000+ elements)
Complex pages with many elements. Must efficiently serialize and transmit. May need to virtualize or paginate.

## Rapid mutations
Animations, timers, real-time updates causing many mutations per second. Must debounce or batch updates.

## Slow network
Page still loading, elements appearing over time. Agent must handle partial state and wait for stability.


---

# Error States
## Element disappeared
Agent tries to click element that no longer exists. Must report error and provide updated state.

## Page crashed or unresponsive
Browser tab becomes unresponsive. Must detect and report, possibly recover.

## Navigation during action
Page navigates while agent is performing action. Must handle gracefully and report new state.

