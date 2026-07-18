# Default Sidebar, Fixed Meter, and DPS Label Design

## Goal

Keep the management navigation visible by default, make the compact damage meter reliably show all four party rows without scrollbars, and replace the mistranslated Korean damage-per-second label with `DPS`.

## Management Sidebar

- The desktop sidebar starts expanded whenever the management window is created.
- The existing burger button continues to collapse and reopen it.
- Mobile-width layouts remain collapsed by default to preserve content space.
- No sidebar state is persisted between launches.

## Compact Meter Geometry

- The meter uses the existing 1920x1080 reference geometry of `330x145` at `x45/y470`.
- Monitor scaling continues to use the existing `0.75` to `1.5` scale range.
- The meter window is not user-resizable, preventing restored undersized geometry from clipping rows or creating scrollbars.
- The document and meter root hide overflow. The compact meter still renders at most four rows.
- The meter header remains rendered while waiting for combat so the visible window can always be dragged when click-through is disabled.
- Resetting window geometry restores the scaled reference size and position.

## Korean DPS Label

- The Korean `ui.logs.damage-per-second` translation changes from the typo `초당 메디지` to `DPS`.
- Every graph title, legend, and related view using that translation key changes together.
- Other languages and existing compact-meter `DPS` labels remain unchanged.

## Testing

- A layout test proves the desktop sidebar initializes expanded while the mobile sidebar remains collapsed.
- A window configuration test proves the meter is non-resizable and retains `330x145` reference geometry.
- A meter rendering test proves the waiting header remains visible and the meter root prevents overflow.
- A localization test proves the Korean damage-per-second label is `DPS` and the typo is absent.
- Run formatting, linting, TypeScript checks, frontend tests, production build, Rust workspace tests, release hook build, MSI packaging, and SHA-256 equality checks required by `AGENTS.md`.

## Non-goals

- Persisting sidebar collapsed/expanded state.
- Dynamically changing meter height based on party size.
- Changing damage calculations or graph data.
- Changing mobile navigation behavior.
