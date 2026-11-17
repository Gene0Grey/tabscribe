---
name: tabscribe
description: The Chrome tab you are watching is a live tile; one press turns it into text.
colors:
  ground: "#000000"
  ink: "#ffffff"
  ink-dim: "rgba(255, 255, 255, 0.68)"
  ink-faint: "rgba(255, 255, 255, 0.46)"
  rule: "rgba(255, 255, 255, 0.14)"
  cobalt: "#1ba1e2"
  cobalt-fill: "#0e73b3"
  cobalt-press: "#0c639a"
  magenta: "#d80073"
  tile-empty: "#161616"
typography:
  pivot:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "4.5rem"
    fontWeight: 300
    lineHeight: 1.1
    letterSpacing: "-0.02em"
  display:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "3rem"
    fontWeight: 300
    lineHeight: 1
    letterSpacing: "-0.03em"
  headline:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "2rem"
    fontWeight: 350
    lineHeight: 1.12
    letterSpacing: "-0.01em"
  title:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1.85rem"
    fontWeight: 300
    lineHeight: 1.12
    letterSpacing: "-0.015em"
  row-title:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 300
    lineHeight: 1.2
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 350
    lineHeight: 1.55
  hint:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1rem"
    fontWeight: 350
    lineHeight: 1.45
  label:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "0.95rem"
    fontWeight: 350
  timestamp:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "0.95rem"
    fontWeight: 400
    fontFeature: "tnum"
  pivot-short:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "3.4rem"
    fontWeight: 300
  pivot-narrow:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "4rem"
    fontWeight: 300
  display-short:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "2.5rem"
    fontWeight: 300
  tile-action:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1.9rem"
    fontWeight: 300
  headline-short:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1.65rem"
    fontWeight: 350
  notice:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1.25rem"
    fontWeight: 350
  input:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "1.05rem"
    fontWeight: 350
  footer:
    fontFamily: "Selawik, Segoe UI Variable Text, Segoe UI, sans-serif"
    fontSize: "0.85rem"
    fontWeight: 350
rounded:
  none: "0px"
spacing:
  page: "48px"
  page-narrow: "20px"
  pivot-gap: "30px"
  section: "30px"
  under: "12px"
components:
  source-tile:
    backgroundColor: "{colors.cobalt-fill}"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "16px 20px 18px"
    height: "168px"
    width: "100%"
  source-tile-hover:
    backgroundColor: "#1081c7"
  source-tile-busy:
    backgroundColor: "{colors.cobalt-fill}"
    textColor: "{colors.ink}"
  source-tile-fail:
    backgroundColor: "{colors.magenta}"
    textColor: "{colors.ink}"
  source-tile-empty:
    backgroundColor: "{colors.tile-empty}"
    textColor: "{colors.ink}"
  button-outline:
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "0 16px"
    height: "44px"
  button-outline-hover:
    textColor: "{colors.cobalt}"
  button-quiet:
    textColor: "{colors.cobalt}"
    padding: "0 6px"
    height: "44px"
  input:
    backgroundColor: "{colors.ground}"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "0 14px"
    height: "44px"
  pivot-item:
    textColor: "{colors.ink-faint}"
    typography: "{typography.pivot}"
  pivot-item-active:
    textColor: "{colors.ink}"
  tool:
    textColor: "{colors.ink}"
    padding: "0 10px"
    height: "34px"
  history-row:
    textColor: "{colors.ink}"
    padding: "14px 0 15px"
    width: "100%"
---

# Design System: tabscribe

## Overview

**Creative North Star: "The Live Tile"**

A Metro typographic world, distilled. Pure black ground, white Selawik set light and lowercase, and exactly one solid colour block on screen: the cobalt tile that holds the Chrome tab you are already watching. Its whole surface is the action. Everything else is type on black: the pivot section names at the top are the navigation, a single hint line sits under the tile, and the transcript reads below a hairline.

The world was first built as a three-pane panorama and then distilled at the user's request because it read as cluttered. What shipped is one column (max 880px of content) with a pivot between "transcribe" and "history". The restraint is the system: one filled tile, one accent hue, magenta only when something failed. Hierarchy comes from size and weight of type, not from boxes, cards, or colour.

Flat, square, and still until something is live. Motion is Metro's: content glides in from the right on a steep decelerating curve, the tile flips down into place, and indeterminate progress is the row of rushing dots.

**Key Characteristics:**
- Black ground, white light type, lowercase throughout.
- One solid cobalt tile per screen; it is the primary action.
- Zero radius, zero shadow, zero gradient.
- 2px white outlined controls that turn cobalt on hover.
- Pivot navigation set as giant light words, inactive ones faded.
- Metro motion: translate-in from the right on `cubic-bezier(0.1, 0.9, 0.2, 1)`.

## Colors

A black-and-white typographic palette with a single cobalt accent and one reserved alarm colour.

### Primary
- **Metro Cobalt** (`cobalt`): the accent as marks and text on black, 7.3:1. Timestamps, the quiet action links ("paste a link", "change"), input focus border, and every hover state.
- **Cobalt Tile Fill** (`cobalt-fill`): the same hue deepened to carry white text at 5.1:1. Fills the source tile (idle and working) and the search-match highlight in the transcript. Never used as text.

### Secondary
- **Metro Magenta** (`magenta`): the failure tile only. White text on it reads 5.0:1. It never appears as text, border, or decoration.

### Neutral
- **Black Ground** (`ground`): the page, and the fill inside inputs.
- **White Ink** (`ink`): primary text, control outlines, progress fill, focus ring.
- **Dim Ink** (`ink-dim`, 8.5:1): hints, meta lines, the footer, the search icon.
- **Faint Ink** (`ink-faint`, 4.6:1): inactive pivot words only; large text only.
- **Hairline** (`rule`): the 1px divider above the transcript and between history rows.
- **Empty Tile** (`tile-empty`): the source tile when no browser tab was found, so the slot stays visible without claiming an action.

### Named Rules
**The One Tile Rule.** At most one solid colour block is on screen at a time, and it is the source tile. Its colour is its state: cobalt fill is "ready / working", magenta is "failed", near-black is "nothing to do". No other surface gets a fill.

**The One Accent Rule.** Cobalt is the only accent hue. It marks what is live or actionable: links, timestamps, hover, focus, matches. If something is cobalt, it can be pressed or it is happening now.

**The Magenta Is Alarm Rule.** Magenta appears only as the failure tile. Never as an accent, a highlight, or decoration.

## Typography

**Display Font:** Selawik (with Segoe UI Variable Text, Segoe UI, sans-serif), embedded as woff2 at weights 300, 350, 400, 600.
**Body Font:** the same family; a single-family system.

**Character:** Segoe-metric humanist sans, set light. Big text goes thinner (300) and tighter (negative tracking); running text sits at 350. Lowercase copy throughout, including headings and buttons.

### Hierarchy
- **Pivot** (300, 4.5rem, 1.05, -0.035em): section names that are also the navigation, set at Metro header scale. Inactive in faint ink, active in white; the inactive item may crop off the right edge on narrow windows.
- **Display** (300, 3rem, 1, -0.03em): the stage word inside the working tile ("fetching", "transcribing").
- **Headline** (350, 2rem, 1.12, -0.01em): the tab title inside the tile, clamped to two lines, max 34ch. The tile's action word sits at 300, 1.9rem.
- **Title** (300, 1.85rem, 1.12, -0.015em): the transcript title, single line with ellipsis.
- **Row Title** (300, 1.5rem, 1.2): history entries.
- **Body** (350, 1.125rem, 1.55, max 70ch): transcript lines, in 92% white.
- **Hint** (350, 1rem, 1.45): the line under the tile, in dim ink. A larger 1.25rem variant (max 40ch) carries messages that stand alone, such as the empty history note and the failure detail.
- **Label** (350, 0.92 to 0.95rem): source host line, meta lines, toolbar tools. Footer at 0.85rem.
- **Timestamp** (400, 0.95rem, tabular numerals): cobalt, in a 72px gutter beside each line.

### Named Rules
**The Light At Size Rule.** The bigger the type, the lighter and tighter it gets: 300 weight with negative tracking from 1.5rem up; 350 for reading sizes. Weight 600 is reserved for the one fact worth bolding in a line (the active model name in the footer).

**The Lowercase Voice Rule.** Interface copy is lowercase: pivot, buttons, hints, states. Proper content (video titles) keeps its own casing.

## Layout

A single column. The pivot runs across the top at 26px from the top with a 30px gap between words. Below it, the page column is capped at 880px of content plus 48px side padding, left-aligned, and fills the remaining height. Order inside "transcribe": source tile (full width, 168px tall), the under-line (12px below, min 48px tall: hint on the left, actions on the right), then the transcript 30px further down behind a hairline, which owns all remaining height and scrolls internally. "history" is a single scrolling list of rows. A footer line pins to the bottom with the privacy statement left and the model choice right.

The window never scrolls; only the transcript lines and history list do (6px square scrollbar thumb).

Responsive behaviour: short windows (height under 700px) tighten top padding to 16px, shrink the pivot to 2.25rem and the tile to 140px. Snapped beside the browser (width under 640px) the side padding drops to 20px, the pivot to 2.1rem, the hint takes its own row above the actions, the footer stacks, and the toolbar wraps.

## Elevation & Depth

Completely flat. No shadows anywhere, no gradients, no blur. Depth is expressed only by the one filled tile against black and by the hairline that separates the transcript from the controls above. Motion supplies the only sense of space: pages slide in from 48px right, new transcript lines glide in from 32px right, and the tile flips down from a hinged top edge on arrival.

### Named Rules
**The Flat Metro Rule.** No box-shadow, no gradient, no translucent glass. If something needs separation, use the 1px hairline or the tile fill.

## Shapes

Every corner is square (0 radius), including inputs, buttons, focus rings, scrollbar thumbs, progress bars and the progress dots (4px squares). Controls are drawn with a 2px solid stroke. Icons are 24-unit SVGs stroked at 1.6 with square caps, or solid for play and stop, rendered at 14 to 18px.

## Components

### Source Tile (signature)
The current Chrome tab presented as a full-width Metro tile; the whole tile is one button.
- **Shape:** square, full width, 168px tall, padding 16px 20px 18px.
- **Content:** small source line (host, 86% white), the tab title as headline, and the action word "transcribe" with a solid play glyph anchored bottom right.
- **Hover:** fill brightens one step. **Focus:** 2px white ring at 3px offset.
- **Arrival:** flips down from its top edge (rotateX -80deg to 0, 0.6s, Metro ease).
- **Working:** stays cobalt; the stage word goes to display size, a detail line sits under it, then a 4px progress track (28% white) fills in white via scaleX. With no measurable progress, five 4px white dots rush across in Metro's indeterminate pattern.
- **Failed:** magenta fill, "couldn't transcribe" as the source line, the plain-language message at 1.25rem.
- **Empty:** near-black fill with a cobalt dot halftone in the top-right third, telling the person to open a video in their browser.

### Buttons
- **Shape:** square, 2px white stroke, 44px tall, 0 16px padding, 1.1rem lowercase label, optional 14px solid icon after the label.
- **Hover:** stroke and label turn cobalt (0.15s). **Disabled:** 38% opacity.
- **Quiet:** no visible stroke, cobalt label, underlines on hover. Used for secondary moves ("paste a link", "cancel", "dismiss").

### Toolbar Tools
Text-and-icon actions beside the transcript title (copy, save txt, save srt, open): 34px tall, 0.92rem, no stroke, turn cobalt on hover, 38% when disabled.

### Inputs / Fields
- **Style:** square, black fill, 2px stroke at 82% white, 44px tall, placeholder at 56% white.
- **Focus:** stroke turns cobalt; no glow.
- **Search variant:** 40px tall, max 360px wide, 40% white stroke, stroked search icon inset left in dim ink.

### Navigation
The pivot: two giant light words, "transcribe" and "history". Inactive in faint ink, dim on hover, white when active. No underline, no tab bar, no indicator; the change in brightness is the selection. Switching slides the page in from the right.

### Transcript Lines
A two-column grid: 72px cobalt timestamp (hover turns white and underlines; it seeks), then the line at body size. Search matches are highlighted in cobalt fill under white text. While transcribing, the newest line glides in.

### History Rows
Full-width buttons separated by hairlines: light 1.5rem title (cobalt on hover), dim meta line under it with host, length and age.

## Do's and Don'ts

### Do:
- **Do** keep exactly one filled tile on screen, and let its fill state be the status.
- **Do** use cobalt for anything live or pressable, and nothing else.
- **Do** set large type at weight 300 with negative tracking, and reading text at 350.
- **Do** keep every corner square and every control stroke at 2px.
- **Do** separate regions with the 1px hairline or with space, never with a box.
- **Do** move things in from the right on `cubic-bezier(0.1, 0.9, 0.2, 1)`, and drop all motion under reduced-motion.
- **Do** keep the hint line to one plain-language sentence in dim ink.

### Don't:
- **Don't** add a second filled tile, card, or panel; the panorama was distilled away for being cluttered.
- **Don't** use magenta for anything but the failure tile.
- **Don't** introduce a second accent hue.
- **Don't** use shadows, gradients, or rounded corners.
- **Don't** put faint ink (46% white) on text smaller than the pivot.
- Responsive steps: pivot 3.4rem / display 2.5rem / headline 1.65rem on windows under 820px tall; under 640px wide the pivot goes back up to 4rem so the inactive section crops off the right edge (Metro's promise of more). The tile has no fixed height: it is as tall as its words, so the transcript keeps the room. Search sits in the toolbar row, and toolbar labels collapse to icons under 640px.
- The already-transcribed tile uses the tile ground (#161616) with "transcribed" as its big word; cobalt fill is reserved for things you can press or that are happening now. The tile's "transcribe" action label is 1.9rem light; notices (empty, error copy, history empty) 1.25rem; inputs 1.05rem; footer 0.85rem. The action tile darkens to cobalt-press (#0c639a) on hover so white text keeps contrast.
- **Don't** add labels above titles. Inside the tile the big words come first and the small facts (host, status, line count) sit under them, bottom left, in pure white.
