# 01 — Design System R1

**Status:** Proposed design tokens; visual ratification pending  
**Target:** Windows desktop, Alpha 0.1

## 1. Design principles

- Calm, light, information-dense but readable.
- Desktop-first; keyboard and pointer interactions are first-class.
- Business state is explicit; color is never the sole state indicator.
- Prefer predictable native-feeling controls over decorative effects.
- Support offline operation without making remote services look mandatory.
- Keep AI affordances visually distinct from authoritative business data.

## 2. Color tokens

| Token | Value | Usage |
|---|---|---|
| color-primary-600 | #2563EB | Primary action, selected navigation |
| color-primary-700 | #1D4ED8 | Hover/pressed |
| color-primary-050 | #EFF6FF | Selected/soft primary surface |
| color-app-bg | #F5F7FA | Application background |
| color-surface | #FFFFFF | Cards, dialogs, drawers |
| color-surface-subtle | #F8FAFC | Secondary panels |
| color-surface-hover | #F1F5F9 | Hover rows |
| color-text-primary | #172033 | Primary text |
| color-text-secondary | #64748B | Secondary text |
| color-text-muted | #94A3B8 | Tertiary text |
| color-border | #E2E8F0 | Default border |
| color-border-strong | #CBD5E1 | Emphasized border |
| color-success | #15803D | Success |
| color-warning | #D97706 | Warning |
| color-danger | #DC2626 | Destructive/error |
| color-info | #0369A1 | Informational state |

Status colors must be paired with labels/icons. Do not encode status using color alone.

## 3. Typography

Use system font stacks to avoid downloading fonts at runtime.

Suggested stack:
`"Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", sans-serif`

| Token | Size / line height | Usage |
|---|---|---|
| type-page-title | 24 / 32 px | Page title |
| type-section-title | 16 / 24 px | Section heading |
| type-body | 14 / 20 px | Main content |
| type-body-small | 13 / 18 px | Dense lists |
| type-caption | 12 / 16 px | Metadata |
| type-overline | 11 / 16 px | Group labels |
| type-metric | 28 / 34 px | Dashboard metrics |

Use font weight 400 for body, 500 for controls, 600 for headings. Avoid excessive bold text.

## 4. Spacing and sizing

Base unit: 4 px.

| Token | Value |
|---|---:|
| space-1 | 4 px |
| space-2 | 8 px |
| space-3 | 12 px |
| space-4 | 16 px |
| space-5 | 20 px |
| space-6 | 24 px |
| space-8 | 32 px |
| space-10 | 40 px |
| space-12 | 48 px |

Control heights: compact 28 px, default 36 px, large 40 px. Minimum interactive target should be 32 × 32 px on desktop where practical.

## 5. Radius, border and elevation

| Token | Value | Usage |
|---|---:|---|
| radius-sm | 6 px | Inputs, badges |
| radius-md | 8 px | Cards, menus |
| radius-lg | 12 px | Dialogs, major panels |
| radius-pill | 999 px | Status pills |
| border-default | 1 px solid #E2E8F0 | Standard surfaces |
| shadow-popover | 0 8px 24px rgba(15,23,42,.10) | Menus/popovers |
| shadow-dialog | 0 16px 48px rgba(15,23,42,.16) | Dialogs |

Do not apply shadows to every card. Use borders and surface contrast as the primary hierarchy.

## 6. Layout grid

- App header: 56 px.
- Sidebar expanded: 240 px (proposed).
- Sidebar collapsed: 64 px (proposed).
- Context panel: 360 px (proposed; future capability).
- Main content max width: 1440 px for reading-oriented pages; project boards may use full available width.
- Main page padding: 24 px; compact mode 16 px.
- Card gap: 16 px; section gap: 24 px.

These are design targets, not verified minimum-window constraints. Validate at 1280 × 720 and 1440 × 900.

## 7. Component visual rules

### Buttons
- Primary: one dominant action per page/dialog.
- Secondary: neutral border/surface.
- Tertiary: text or subtle icon button.
- Destructive: danger treatment and explicit confirmation for irreversible effects.
- Disabled state must explain unavailable prerequisites where useful.

### Inputs
- Always show labels; placeholder is not a label.
- Validation message appears adjacent to the relevant field.
- Preserve user input after recoverable errors.
- Distinguish read-only, disabled and loading.

### Cards and lists
- Use consistent title, metadata and action placement.
- Row hover must not imply selection.
- Selection and focus need distinct visual states.

### Badges
- Short text labels; avoid status-only iconography.
- Use neutral treatment for ordinary metadata.
- Do not overload colors with multiple meanings.

### Dialogs and drawers
- Dialog for focused decisions or destructive confirmation.
- Drawer for inspect/edit workflows that benefit from preserving page context.
- Escape closes only when no unsaved destructive state would be lost; otherwise prompt.

## 8. Accessibility

- Keyboard focus visible at all times.
- Logical tab order follows visual order.
- Every icon-only button has an accessible name and tooltip.
- Text contrast target WCAG 2.1 AA.
- Form errors are programmatically associated with fields.
- Drag-and-drop has keyboard-operable alternatives.
- Respect Windows reduced-motion preference where exposed by the runtime.

## 9. Motion

Use restrained transitions: 120–180 ms for hover/focus, 160–220 ms for drawer/dialog. Avoid animated layout changes that obscure data updates. Motion is supplementary and must not communicate the only state change.

## 10. Token implementation

Tokens should be implemented as CSS custom properties in the prototype and later mapped to the production styling system. Do not duplicate literal colors across feature stylesheets.
