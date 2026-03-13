# Success Criteria

What does success look like for Agent Browser Protocol?


---

# Developer Experience
- 3 steps or less to get running (install, start, connect)
- No browser extensions required
- Works with any Chromium browser
- Agent-agnostic — works with OpenClaw, Claude, Codex, custom agents
- TypeScript SDK with full type definitions

---

# Perception Quality
- Agent sees everything a human sees (colors, fonts, spacing, borders)
- Streaming updates — agent aware of changes in real-time
- Computed styles, not CSS — what is actually rendered
- Accessibility data included (contrast ratios, focus states, ARIA)
- Element references stable across mutations

---

# Action Reliability
- Actions reference elements by stable ID, not coordinates
- Actions can also reference by role + label (semantic)
- Actions visible in real-time (human can watch agent work)
- Action results reported back (success/failure)

---

# Performance
- PageState payload < 50KB for typical page (vs ~500KB screenshot)
- Mutation updates < 5KB (incremental, not full refresh)
- Latency < 100ms from page change to agent notification
- Works on pages with 1000+ elements

---

# Use Case Validation
- Agent can detect WCAG accessibility violations
- Agent can notice visual regressions (color changes, misalignment)
- Agent can complete multi-step user flows (sign up, checkout)
- Agent can explore pages without predefined scripts
