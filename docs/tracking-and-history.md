# Tracking and Live History

This document defines how `winproc-tui` represents tracking intent, process identity, named Tracking Lists, and bounded Live history. Metric meanings remain in [metrics.md](metrics.md), Graph ownership remains in [graph-workspace.md](graph-workspace.md), and Recording scope remains in [recording-and-log-view.md](recording-and-log-view.md).

## Concepts

| Concept | Meaning |
|---|---|
| Tracking List entry | A case-insensitive process name expressing what the user wants to retain or record. |
| `ProcessIdentity` | One process lifetime identified by PID, name, and start time. |
| Working Tracking List | The mutable set used by the current Live session. |
| Saved named Tracking List | A persistent definition changed only by explicit list-management actions. |
| Tracked-only | An independent display filter; it is not inferred from whether the working list is empty. |
| Ghost Row | The newest exited identity retained for a tracked name so its final values and history remain inspectable. |

Tracking intent uses process names because PIDs change across restarts and one name can have several live instances. Histories, selections, Process Info targets, and process Graphs use full `ProcessIdentity` values so a reused PID or restarted process never inherits another lifetime's samples.

System history is independent from Tracking Lists. MEM, GPU, System Activity, and aggregate CPU histories are retained without registering a process name.

## Working and Saved Lists

Changing a process's tracked state edits only the working Tracking List. Saved named definitions change only through explicit Save, Save As, Rename, or Delete actions.

The Tracking Lists dialog includes a virtual `Empty (default)` entry. It represents an empty working list, is active only when no saved definition is active, and is never persisted, renamed, deleted, or overwritten. Loading it or a saved definition replaces the working list without changing Tracked-only.

Loading a definition can remove names whose older retained samples are no longer needed. When that operation would discard history beyond general Live retention, the application asks for confirmation before pruning it.

## Startup

Startup mode can resume the previous working list, start empty, or open a chooser containing the previous working list, the virtual empty entry, and saved definitions.

The startup choice is resolved before the first sample. This ensures tracked-history retention applies from the first capture. Canceling the chooser exits before initial sampling and restores the terminal.

Tracking List startup changes and explicit saved-list actions persist immediately. Other session settings are written after a successful interactive run; filter input is never persisted.

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
- Working-list changes must not overwrite saved definitions implicitly.
- The virtual empty entry must never be persisted as a named definition.
- History pruning must remove samples and peaks together.
- Ordinary Live history must retain at most two complete generations per case-insensitive process name.
- Concurrently live identities and explicit paused-display, Graph, or Process Info references must remain inspectable even when they exceed the ordinary generation limit.
- A paused Ghost Row, registered process Graph, or open Process Info target must remain inspectable even when its identity would otherwise age out.
