# Handoff: EQ Box

Written 2026-10-14. Read CLAUDE.md, then this.

## Where it stands

- This repository was recreated on 2026-10-14. The original EQ-Box repository was renamed TAW-Equaliser (the hi-fi family's equaliser, new look). EQ Box itself continues here as the **Box family's** equaliser, with Drone Box's Jupiter panel.
- Branch `claude/focused-ritchie-veg4m9` holds EQ Box's full history up to the **0.2.0 release** (`97a9c91`, the merge of "Release 0.2.0"), plus README/CHANGELOG/CLAUDE.md notes on its place in the family. It's self-contained: its own panel code and the SP-10 Spectrum Analyser, no TAW-Panel dependency.
- There is no `main` yet. When it's set to this branch, `.github/workflows/release.yml` will publish v0.2.0 here (the tags from the old repository didn't come across; tag pushes are blocked from cloud sessions anyway, and the workflow makes its own).

## Watch out for

- **Same Windows engine as TAW Equaliser** (same CLSID). Only one can be installed; the README says so. If EQ Box's install script is touched, consider making it detect a TAW Equaliser engine, as TAW Equaliser's detects EQ Box's.
- The `Drum Box` and a matching "Box Analyser" were mentioned as family ideas; SP-10 inside EQ Box already plays that part.

## Next

1. Set `main` (merge this branch) so the repository has a default branch and a release.
2. Any Box-family changes from here: keep the Jupiter look (it's Drone Box's), and keep EQ Box's names on disk (`EQ Box` folders, `eq-box` programs).
