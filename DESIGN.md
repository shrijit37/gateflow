---
name: Gateflow
description: Self-deployed streaming workflow gateway — minimal, fast, utilitarian.
colors:
  primary: "#2563eb"
  primary-hover: "#1d4ed8"
  ingress: "#059669"
  egress: "#7c3aed"
  stream-text: "#6ee7b7"
  paper: "#ffffff"
  ink: "#171717"
  paper-dark: "#0a0a0a"
  ink-dark: "#ededed"
  line: "#e4e4e7"
  line-strong: "#d4d4d8"
  line-dark: "#27272a"
  line-strong-dark: "#3f3f46"
  muted: "#71717a"
  wash: "#fafafa"
  code-fill: "#f4f4f5"
  panel-dark: "#18181b"
  stream-fill: "#09090b"
  success-bg: "#d1fae5"
  success-text: "#065f46"
  warning-bg: "#fef3c7"
  warning-text: "#92400e"
  danger: "#dc2626"
typography:
  headline:
    fontFamily: "Geist, Arial, Helvetica, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 700
    lineHeight: 1.2
  title:
    fontFamily: "Geist, Arial, Helvetica, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 600
    lineHeight: 1.4
  body:
    fontFamily: "Geist, Arial, Helvetica, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Geist, Arial, Helvetica, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "0.05em"
  mono:
    fontFamily: "Geist Mono, ui-monospace, monospace"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.6
rounded:
  xs: "4px"
  md: "6px"
  lg: "8px"
  full: "9999px"
spacing:
  sm: "8px"
  md: "16px"
  lg: "24px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.paper}"
    typography: "{typography.title}"
    rounded: "{rounded.md}"
    padding: "6px 12px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    typography: "{typography.title}"
    rounded: "{rounded.md}"
    padding: "6px 12px"
  input:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
---

# Design System: Gateflow

## Overview

**Creative North Star: "The Switchboard"**

Gateflow's interface is the switchboard around the stream, not the stream itself. It is an operator console in two shells: a calm centered list for creating workflows, and a dense full-height builder for wiring, deploying, and verifying them. Every pixel serves task completion — create, wire, deploy, verify — and nothing is decorative.

Color is signal, never ornament. The only strong hues on any screen are the deep emerald and violet node-kind bars on the canvas and the emerald-on-black stream output in the test drawer, plus the single blue primary action. Everything else is zinc hairlines and tonal fills, in light and dark alike.

**Key Characteristics:**
- Flat hairline surfaces; only canvas nodes lift off the page.
- One blue action per screen; ghost buttons for everything else.
- Deep emerald = ingress, violet = egress — the two signals.
- Geist Mono for anything the machine said; Geist for everything human.
- Dense ops-console builder versus calm centered list.

## Colors

Zinc neutrals carry the interface; blue marks the single primary action; deep emerald and violet mark node kind.

### Primary
- **Gateway Blue** (#2563eb): the one primary action per screen (Create, Deploy, Send) and text links. Hover deepens to #1d4ed8.

### Secondary
- **Ingress Emerald** (#059669): ingress node kind bar and its canvas handles.
- **Egress Violet** (#7c3aed): egress node kind bar and its canvas handles.
- **Stream Signal** (#6ee7b7 on #09090b): live stream output text on near-black.

### Neutral
- **Paper** (#ffffff) / **Ink** (#171717): light-mode surface and text.
- **Paper Dark** (#0a0a0a) / **Ink Dark** (#ededed): dark-mode surface and text.
- **Hairline** (#e4e4e7) / **Hairline Strong** (#d4d4d8): borders; dark equivalents #27272a / #3f3f46.
- **Muted** (#71717a): secondary text, hints, slugs, section labels.
- **Wash** (#fafafa), **Code Fill** (#f4f4f5), **Panel Dark** (#18181b), **Stream Fill** (#09090b): tonal fills for status strip, code blocks, and stream output.
- **Deployed tint** (#d1fae5 on #065f46) / **Draft tint** (#fef3c7 on #92400e): status badges.
- **Danger** (#dc2626): error text only.

### Named Rules
**The Two-Signal Rule.** Only emerald and violet carry meaning (node kind), and blue acts. Nothing else gets hue: status is tinted badge backgrounds, errors are red text, everything structural is zinc.

## Typography

**Display Font:** Geist (with Arial, Helvetica, sans-serif fallback)
**Body Font:** Geist (with Arial, Helvetica, sans-serif fallback)
**Label/Mono Font:** Geist Mono (with ui-monospace, monospace fallback)

**Character:** A neutral grotesk set with operator confidence — semibold labels, tabular numerals in stats — paired with a mono face that handles every machine utterance, from slugs to stream chunks.

### Hierarchy
- **Headline** (700, 1.5rem, 1.2): the page title ("Gateflow") on the list screen. The builder has no headline; its header is chrome, not prose.
- **Title** (600, 0.875rem, 1.4): section heads, node subtitles, button labels, drawer title.
- **Body** (400, 0.875rem, 1.5): descriptions, hints, empty states, panel help text.
- **Label** (600, 0.75rem, uppercase, 0.05em tracking, muted): drawer section heads, node kind bars.
- **Mono** (400, 0.75rem, up to 1.6; 11px in code blocks): slugs, endpoint URLs, curl equivalents, deploy stats, stream output.

### Named Rules
**The Mono-For-Machines Rule.** Anything addressable, copyable, or emitted — slugs, URLs, curl, stats, chunks — is Geist Mono. Human prose never is.

## Layout

Two shells, one rhythm. The list is a calm centered column (max-w-3xl, 24px sides, 40px top). The builder is a full-height ops console: wrapping header bar, status strip, then canvas plus fixed side panels (config aside 320px, test drawer 480px). Density is compact on an 8px base rhythm (8/12/16px paddings, 8/12px gaps). The create form stacks below the sm breakpoint (640px); the builder header wraps instead of collapsing.

## Elevation & Depth

Flat by default. Depth is conveyed through hairline borders and tonal fills, never shadows — except flow nodes, which lift slightly off the canvas. Selection is a blue ring, not a deeper shadow. Dark mode keeps the same borders; fills step down to zinc-900/950.

### Shadow Vocabulary
- **Node rest** (`box-shadow: 0 1px 2px rgba(0,0,0,0.05)`): flow nodes at rest, light mode only.

### Named Rules
**The Flat-By-Default Rule.** Surfaces never cast shadows. Only canvas nodes lift, and only barely.

## Shapes

Controls are gently rounded (6px inputs, buttons, selects); containers and nodes step up to 8px, with the node kind bar rounding the top corners only; status badges go full-round; code blocks and stream output stay near-sharp at 4px. Every edge is a 1px zinc hairline.

### Named Rules
**The 4/6/8 Rule.** Machine surfaces 4px, interactive controls 6px, containers 8px, status full-round. Nothing else rounds.

## Components

### Buttons
Utilitarian pair, no shadows, instant state change. **Shape:** gently rounded (6px), 6px/12px padding, 0.875rem semibold. **Primary:** Gateway Blue fill, white text; hover deepens the blue. **Ghost:** transparent with 1px zinc border and zinc text; hover washes zinc-100. Focus is a visible blue outline.

### Inputs / Fields
Quiet boxes that sharpen on focus. **Style:** white fill (zinc-900 dark), 1px zinc border, 6px radius, 6px/10px padding, 0.875rem. **Focus:** border shifts to Gateway Blue, no glow. Mono variant at 0.75rem for slugs and request bodies. **Error:** red helper text below, never a red box.

### Status Badges
Soft full-round pills, tinted backgrounds with dark tinted text: emerald tint for deployed, amber tint for draft. 12px medium, 2px/8px padding.

### Flow Nodes (signature)
White cards (zinc-900 dark) with a solid kind-color title bar — deep emerald for ingress, violet for egress — carrying the kind in 0.75rem semibold white caps, then the node name (0.875rem medium) and live config summary (0.75rem muted, mono for method/host). **Shape:** 8px card, title bar rounds top only. **Selected:** blue border plus blue ring. Handles inherit the kind color.

### Code / Stream Surfaces
Near-sharp 4px blocks: curl equivalents on zinc-100 (zinc-900 dark) in 11px mono; the stream output is the room's dark corner — near-black fill, emerald stream text, relaxed 1.7 line height, auto-scrolling.

### Builder Header
A wrapping chrome bar: back link, name input, mono slug, version, status badge, and right-aligned ghost actions with the single blue Deploy. Deploy/save outcomes land in a mono status strip directly beneath it.

## Do's and Don'ts

### Do:
- **Do** keep one blue primary action per screen; everything else is ghost.
- **Do** set machine text (slugs, URLs, curl, stats, chunks) in Geist Mono.
- **Do** reserve emerald and violet for ingress and egress; deepen, never widen, their use.
- **Do** keep surfaces flat with 1px zinc hairlines; let only canvas nodes lift.
- **Do** follow the 4/6/8 radius scale (machine 4px, controls 6px, containers 8px).

### Don't:
- **Don't** add shadows to cards, panels, drawers, or buttons — nodes are the sole exception.
- **Don't** introduce new hues for status, priority, or decoration; use the deployed/draft tints or zinc.
- **Don't** put the web UI in the streaming path — the builder explains the stream, never sits in it.
- **Don't** set prose in mono or machine output in proportional type.
- **Don't** invent components beyond buttons, inputs, badges, nodes, and code surfaces without a new node kind to justify them.
