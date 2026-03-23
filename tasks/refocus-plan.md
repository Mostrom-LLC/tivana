# Tivana Refocus Plan

## Purpose

This plan resets Tivana around its original goal:

> A standardized way for AI agents to browse the web with human-like awareness through continuous, semantic page understanding, not scripted browser automation.

The repo currently contains meaningful perception work, but the product narrative, public API emphasis, and recent implementation choices have drifted toward general browser automation. This plan defines how to correct that drift without throwing away useful work.

---

## Current Diagnosis

### What still matches the original goal
- The runtime already has page-state, element, accessibility, and mutation primitives.
- The protocol already supports server events and mutation streaming.
- The SDK already exposes an observation-oriented surface, even if it is not the dominant one.
- The documentation still preserves the original perception-first intent in several places.

### Where drift is visible
- The main product story now leads with automation, stealth, CAPTCHA solving, proxy rotation, and batching.
- The SDK is primarily presented as imperative RPC automation methods.
- Recent implementation energy has gone into extension-backed control and browser-operability concerns.
- Test coverage emphasizes scripted flow completion more than exploratory perception and anomaly detection.
- Core docs conflict with one another on whether extensions exist and whether screenshots are in or out of scope.

### Bottom line
- Tivana currently behaves more like an automation runtime with perception features than a perception-first agent browser protocol.

---

## Product Reset

### Tivana is
- A browser perception protocol for agents.
- A runtime that exposes semantic, continuously updated page understanding.
- A thin action layer that lets agents act on what they perceive.
- A tool for exploratory QA, accessibility review, visual reasoning, and human-observable agent browsing.

### Tivana is not
- A stealth browser automation product.
- A CAPTCHA-solving platform.
- A scraping toolkit.
- A generic RPA replacement.
- A bot-evasion brand.

### Positioning statement
- Tivana gives agents structured, streaming awareness of the rendered web page so they can explore, notice, judge, and act semantically.

### Design principle
- Perception is the product.
- Actions exist to support perception-driven agency, not to become the center of the system.

---

## Scope Boundaries

### Must be first-class
- `perceive.pageState`
- `perceive.elements`
- `perceive.accessibilitySnapshot`
- `perceive.mutations`
- Stable element identity within a session
- Semantic actions by element identity and role/label
- Human-observable browser behavior
- Lightweight incremental updates over WebSocket

### Can remain, but must be demoted
- Screenshots
- Raw JS evaluation
- Network inspection
- Tab management
- Storage and cookies
- Extension-backed connection paths

### Should move out of the core story
- Stealth and anti-detection
- CAPTCHA solving
- Proxy rotation
- Speed claims framed around batch automation
- “Zero-config browser automation” language

### Explicit non-goals for the next refocus phase
- Broadening browser support
- Competing with Playwright/Puppeteer on automation breadth
- Adding more anti-bot features
- Shipping more scripted convenience APIs before observation is first-class

---

## Refocus Objectives

### Objective 1: Reposition the product
- Make the repo, docs, and SDK tell one story: agent perception first.

### Objective 2: Make streaming awareness the primary API
- Observation should be the default integration path, not a side helper around imperative requests.

### Objective 3: Align implementation with the story
- Any extension path, screenshots, or automation-heavy helpers must be documented as secondary transports or utilities.

### Objective 4: Validate the intended use cases
- Prove Tivana with exploratory QA, accessibility review, anomaly detection, and judgment-based browsing tasks.

---

## Milestones

## Milestone 1: Narrative and Scope Correction
### Proposed date
- March 30, 2026

### Outcome
- The project’s public surface accurately describes Tivana as a perception-first protocol.

### Deliverables
- Rewrite the root README.
- Add a short “What Tivana Is / Is Not” section to the docs landing path.
- Reconcile doc contradictions about extensions, screenshots, and browser model.
- Demote or remove marketing copy about stealth, CAPTCHA solving, proxies, and automation breadth from primary surfaces.
- Publish a concise protocol overview centered on streaming awareness and semantic updates.

### Owner
- Product/maintainer

### Exit criteria
- README headline and first screen match the original goal.
- No top-level doc presents Tivana primarily as an automation product.
- Docs no longer contradict the current implementation model.

## Milestone 2: Observation-First SDK and Protocol
### Proposed date
- April 13, 2026

### Outcome
- Agents can integrate around an explicit observe-perceive-act loop instead of mostly imperative RPC calls.

### Deliverables
- Define an explicit observation session flow in the protocol.
- Make `perceive.mutations` subscription first-class in the SDK.
- Ensure `observe()` actually starts and manages the mutation stream, rather than only registering callbacks.
- Provide a canonical event model:
  - page loaded
  - navigation committed
  - focused element changed
  - element added/removed/changed
  - viewport or scroll changed
- Tighten payload contracts for incremental updates versus full snapshots.

### Owner
- Runtime + SDK engineering

### Exit criteria
- A new agent integrator can connect, subscribe, receive state, and act without polling glue.
- SDK examples lead with observation, not automation helpers.
- The event model is documented and tested end-to-end.

## Milestone 3: Perception Quality and Identity Semantics
### Proposed date
- April 27, 2026

### Outcome
- The protocol is reliable enough for agents to maintain awareness across real browsing sessions.

### Deliverables
- Audit and tighten element identity stability rules.
- Improve mutation semantics so agents can tell what changed and why it matters.
- Standardize visual-semantic fields that support judgment:
  - role
  - name
  - state
  - bounds
  - relevant computed styles
  - visibility/interactability
- Add quality tests for dynamic pages and SPA transitions.
- Define the minimum “human-like awareness” contract for each page update.

### Owner
- Runtime engineering

### Exit criteria
- Dynamic page changes do not force full re-perception for common cases.
- Agents can reliably follow element identity across small DOM shifts.
- Perception payloads are stable enough for downstream reasoning.

## Milestone 4: Use-Case Validation
### Proposed date
- May 11, 2026

### Outcome
- Tivana is validated on the use cases it was meant to unlock.

### Deliverables
- Replace or supplement automation-centric demos with perception-first demos:
  - accessibility review
  - exploratory QA
  - visual anomaly detection
  - semantic browsing walkthrough
- Add evaluation scripts that test whether an agent can notice and describe anomalies.
- Add benchmark tasks where success depends on judgment, not merely scripted completion.
- Document known limits clearly.

### Owner
- Maintainer + QA

### Exit criteria
- At least three demos show agent judgment rather than only command execution.
- Test artifacts demonstrate perception-led success cases.
- The project can be explained without leaning on stealth or automation claims.

---

## Workstreams

## 1. Documentation
### Priority
- Immediate

### Tasks
- Rewrite [README.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/README.md).
- Add a docs index that points to the perception-first narrative.
- Reconcile [docs/architecture.md](/Volumes/Samsung/repositories/mostrom/node-package-manager/tivana/docs/architecture.md) with the extension-backed implementation.
- Reconcile screenshot language with the original v1 checklist and current codebase.
- Rewrite SDK docs to make `observe()` the front door.

## 2. Protocol and SDK
### Priority
- Immediate

### Tasks
- Make subscription lifecycle explicit.
- Distinguish full snapshots from incremental events.
- Ensure the SDK starts mutation streaming intentionally.
- Add helper APIs for event consumption, not just action invocation.
- Document event ordering and recovery behavior.

## 3. Runtime
### Priority
- Near-term

### Tasks
- Tighten mutation event coverage and semantics.
- Add page-level events beyond DOM mutation.
- Normalize event payloads across direct CDP and extension-backed sessions.
- Clarify whether the extension path is strategic, transitional, or optional.

## 4. Validation
### Priority
- Near-term

### Tasks
- Add exploratory/perception-led test cases.
- Add dynamic-page identity tests.
- Add golden evaluation tasks for anomaly detection and accessibility findings.
- Reduce reliance on scripted success-only browser tests as the main proof point.

---

## Decision Log

### Decision 1
- Tivana will be described as a perception protocol, not an automation framework.

### Decision 2
- Automation-adjacent features may remain in the codebase, but they will not define the product or lead the API narrative.

### Decision 3
- Extension support is allowed only if documented as an implementation transport, not as a contradiction to the core architecture story.

### Decision 4
- The primary quality bar is whether an agent can maintain meaningful awareness and exercise judgment, not whether the runtime can mimic more browser automation features.

---

## Risks

### Risk 1: Existing work loses perceived value
- Mitigation: keep useful implementation pieces, but reclassify them as secondary capabilities.

### Risk 2: The repo continues to send mixed signals
- Mitigation: fix top-level docs first, before adding more features.

### Risk 3: “Observation-first” remains mostly rhetorical
- Mitigation: require a real subscription lifecycle in the SDK and protocol, with tests.

### Risk 4: Validation still rewards scripted automation
- Mitigation: add perception-led acceptance criteria and demos that require anomaly detection or judgment.

### Risk 5: Extension support keeps distorting the architecture
- Mitigation: make a clear call:
  - either it is a temporary compatibility path
  - or it is an official transport that must be reflected in the architecture docs

---

## Acceptance Criteria for the Refocus

- A new reader understands Tivana as a perception-first agent browsing protocol within the first minute.
- The primary SDK example starts with observing page state and reacting to changes.
- The protocol documents full snapshots and incremental updates as first-class concepts.
- The runtime emits enough structured state for agents to notice anomalies without screenshots as the default mechanism.
- Demo and test coverage includes exploratory, judgment-based use cases.
- Stealth/CAPTCHA/proxy functionality no longer dominates the repo’s public identity.

---

## Immediate Next Actions

### This week
- Approve this refocus plan.
- Rewrite the root README and SDK README around perception-first messaging.
- Open a decision issue on extension strategy.
- Open an engineering issue for first-class mutation subscription in the SDK.
- Open a validation issue for perception-led demos and tests.

### Next two weeks
- Ship the doc rewrite.
- Ship the SDK/protocol subscription cleanup.
- Replace at least one automation-centric demo with an exploratory QA or accessibility demo.

---

## Suggested Tracking Issues

- `docs: reposition Tivana as perception-first agent browser protocol`
- `docs: reconcile architecture docs with extension-backed sessions`
- `sdk: make mutation subscription first-class and explicit`
- `protocol: define snapshot vs incremental event contracts`
- `runtime: normalize page and mutation events across transports`
- `qa: add exploratory perception-led evaluation scenarios`
- `demo: add accessibility review walkthrough`
- `demo: add anomaly-detection walkthrough`

---

## Release Readiness Gate for the Refocus

Do not claim the reset is complete until all of the following are true:

- README and docs tell one consistent story.
- SDK onboarding uses observation as the default path.
- Mutation/event streaming works as documented.
- At least one end-to-end example demonstrates agent judgment.
- Maintainers can explain the role of extension support in one sentence without contradiction.
