# Tracking and Live History

This document defines how `winproc-tui` represents investigation state, tracking intent, process identity, Investigation Profiles, and bounded Live history. Metric meanings remain in [metrics.md](metrics.md), Graph ownership remains in [graph-workspace.md](graph-workspace.md), and Recording scope remains in [recording-and-log-view.md](recording-and-log-view.md).

## Concepts

| Concept | Meaning |
|---|---|
| Tracking List entry | A case-insensitive process name expressing what the user wants to retain or record. |
| `ProcessIdentity` | One process lifetime identified by PID, name, and start time. |
| Current Investigation | The mutable, automatically saved investigation state used by the current Live session. |
| Working Tracking List | The tracked-name field inside the Current Investigation. It is not an independently named object. |
| Investigation Profile | The only named reusable investigation unit. It is changed only by explicit profile actions. |
| Tracked-only | A profile-owned display filter; it is not inferred from whether the working Tracking List is empty. |
| Ghost Row | The newest exited identity retained for a tracked name so its final values and history remain inspectable. |

Tracking intent uses process names because PIDs change across restarts and one name can have several live instances. Histories, selections, Process Info targets, and process Graphs use full `ProcessIdentity` values so a reused PID or restarted process never inherits another lifetime's samples.

System history is independent from Tracking Lists. MEM, GPU, System Activity, and aggregate CPU histories are retained without registering a process name.

## Current Investigation and Profiles

The Current Investigation owns the working Tracking List, Tracked-only state, Processes Flat/Tree mode, visible process columns and order, process sort, ordered Graph templates and each Graph's Raw/MA5 mode, Graph layout and time span, Samples and Delta visibility, Y-axis lower-bound mode, and the default Recording interval. It is automatically written as the last investigation after a successful run.

Changing a process's tracked state edits the working Tracking List in the Current Investigation. It marks an active profile as modified but never overwrites that named profile. A saved profile changes only through explicit Save, Save As, or Delete actions; Open explicitly replaces the Current Investigation from a saved profile. Theme, mouse enablement, process column widths, preferred Processes panel height, text filter, selections, retained samples, and runtime process identities remain outside profiles.

Opening a profile is available only in Live and replaces the complete profile-owned portion of the Current Investigation. It can remove names whose older retained samples are no longer needed. When that operation would discard history beyond general Live retention, the application asks for confirmation before pruning it. Profile deletion remains available in Recording and Log view because it does not change the active investigation.

The active profile is the explicit target for Save. A saved profile becomes active only when it is selected at startup, opened in Live, or created with Save As during the current run. `Resume last` and `Start empty` begin with no active profile, so Save follows the Save As flow until a profile is explicitly opened or created. The header shows that binding and a non-color modified marker; Log view has no Current Investigation binding.

Profiles express tracking intent with case-insensitive process names. Their process Graph templates may additionally include an executable-path constraint captured when available. Profiles never store a PID, start time, `ProcessIdentity`, current selection, text filter, retained history, A/B points, or other run-specific cursor state. Graph template resolution is defined in [Graph Workspace](graph-workspace.md).

## Startup

Startup mode can `Resume last`, `Choose Profile`, or `Start empty`. The chooser contains `Last investigation`, `Empty investigation`, and every saved Investigation Profile. The two built-in choices are virtual and are never persisted or bound as named profiles. The startup setting is available from the main menu's Config section.

Startup applies investigation state in two phases. The selected Tracking List, Tracked-only state, process view, columns, sort, Graph workspace options, and Recording default are resolved before the first sample, so tracked-history retention applies from the first capture. After that sample establishes current process and GPU identities, Graph templates resolve against it and receive new run-unique Graph IDs. Unresolved or ambiguous templates are reported and never guessed.

Startup-setting and explicit profile changes persist immediately. Other Current Investigation changes are written after a successful interactive run; filter input is never persisted. Legacy named Tracking Lists are migrated once into Investigation Profiles using the remaining legacy investigation settings. A colliding profile name is preserved by adding a ` (Tracking List)` suffix, with a numeric suffix when needed; subsequent writes omit the legacy list format.

## Processes Flat and Tree Views

The Processes table supports a persisted `Flat` / `Tree` view preference. Flat view keeps the ordinary globally sorted list. Tree view builds a forest from the live rows in the currently displayed snapshot. The sampling worker captures each process's parent PID as part of the normal snapshot, so tree construction does not add work to the UI thread.

A parent edge is accepted only when exactly one live row in that same snapshot has the reported parent PID. The edge targets that row's full `ProcessIdentity`; missing, inaccessible, ambiguous, and self-referential parents become roots. Cycles are broken without recursion, and their members become roots. Parentage is never inferred from an earlier snapshot or from retained history, and it is not added to recording schemas. Log view therefore remains Flat even when Tree is the saved Live preference.

Roots and each sibling group follow the current sort column and direction, then each subtree is displayed in parent-first order. Expand/collapse state is session-local and keyed by full `ProcessIdentity`, so a reused PID does not inherit an earlier process lifetime's state. Leaves, the synthetic Tracked Total row, and Ghost Rows have no disclosure control.

In Tree view, a text filter keeps direct live matches and the ancestor paths needed to locate them. Those ancestors are muted context rows, matching paths are temporarily revealed, and match counts and jump navigation count only direct matches. Expand/collapse is temporarily unavailable while a text filter is active, its disclosure glyphs are muted, and the existing session-local collapsed state resumes unchanged when the filter is cleared. Tracked-only is applied before tree construction: untracked ancestors are not reintroduced, and combining it with a text filter searches only the tracked subset. Ghost Rows remain top-level entries after the live forest and follow the same text filter. Tracked Total remains outside the hierarchy.

Selection follows full process identity across sampling refreshes and sibling reordering. Collapsing a subtree whose descendant has focus moves focus to the collapsed parent, and hidden descendants are removed from multi-selection. Display pause builds the tree from the frozen snapshot; Recording continues to use the current live snapshot and allows Tree view.

## Live History Retention

Tracked process identities retain 7,200 samples, approximately two hours at the fixed one-second Live interval. General non-tracked identities retain 120 samples, approximately two minutes. System history retains 7,200 samples.

Capacity alone is insufficient because frequent process restarts could leave many small identity maps. After every Live snapshot, pruning retains:

- the two newest ordinary identities for each case-insensitive process name, selected from identities sampled within general Live retention and retained exits for tracked names;
- every current process identity, including all concurrently live same-name instances;
- Live and Ghost Row identities visible in a paused display;
- identities referenced by process Graphs;
- the fixed target of an open Process Info dialog.

The two-generation limit removes an entire older `ProcessIdentity`; it does not shorten the sample series of either retained generation. Current, paused-display, Graph, and Process Info protections can temporarily retain more than two identities for a name. For a tracked name, up to two exited identities may remain in internal Live state, while the Processes table continues to show only the newest one as its Ghost Row.

Older exited or restarted identities are removed from both sample and peak maps using one retained-identity set. Recording writes every matching generation to the session log independently of this Live pruning. Loaded logs are reconstructed from recorded frames and do not use Live sample or generation capacities.

## Recording Boundary

Starting a Recording copies the working Tracking List into session-owned scope. Later display filtering does not alter that scope, and the working list cannot be edited until Recording ends. See [Recording and Log View](recording-and-log-view.md) for lifecycle rules.

## Invariants

- A tracked name, a currently matching process, and one process identity are distinct concepts.
- PID reuse and process restart must never merge histories.
- Tracked-only must remain independent from the contents of the working Tracking List.
- Current Investigation changes must not overwrite a named profile implicitly.
- Loading a profile must pass the retained-history confirmation boundary before replacing the Current Investigation.
- `Last investigation` and `Empty investigation` must never be persisted as named profiles.
- Tracking intent must be applied before the initial sample; Graph templates must resolve only after current identities exist.
- History pruning must remove samples and peaks together.
- Ordinary Live history must retain at most two complete generations per case-insensitive process name.
- Concurrently live identities and explicit paused-display, Graph, or Process Info references must remain inspectable even when they exceed the ordinary generation limit.
- A paused Ghost Row, registered process Graph, or open Process Info target must remain inspectable even when its identity would otherwise age out.
