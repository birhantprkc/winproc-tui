# Process Investigation

This document defines the ownership and lifecycle of System Info and Process Info investigations. Field meanings and display formats remain in [metrics.md](metrics.md); .NET runtime sampling internals remain in [.NET Runtime Metrics Collection](dotnet-metrics-collection.md).

## System Info

System Info describes the current host rather than a historical sample. Windows product metadata is captured once during startup, while memory, GPU, disk, and CPU capacity values come from the latest Live `Snapshot`.

Display pause and Log view do not replace those host-capacity fields with paused or recorded values. Opening System Info performs no new collection. One ordered field model supplies both rendered rows and complete clipboard output, so terminal clipping never truncates copied data.

## Fixed Process Target

Opening Process Info creates a `ProcessInfoDialogTarget` containing the selected `ProcessIdentity`, opening `ProcessRow`, and lifecycle. Every tab and worker request uses that fixed target instead of consulting the current Processes selection again.

The active tab is retained between ordinary opens. A direct investigation action can select a specific tab for the new dialog session. Opening a new session clears session-local filters, while tab switches and explicit refreshes preserve them.

All live collectors verify that the process still has the expected identity. A PID that exits or is reused must never deliver information to the open dialog.

## Tab Collection Boundaries

| Tab | Data source and lifecycle |
|---|---|
| Metrics | Uses the fixed process identity and its Live, paused, or loaded history. |
| Image | Starts background static-process collection after the target is stable; recorded row data is available as a fallback. |
| Files | Enumerates open disk files on an independent worker on first activation, then refreshes while the tab is visible when collection is inexpensive. |
| DLLs | Takes an explicit module and file-metadata snapshot on its own worker. |
| Environment | Reads the live target's remote environment block on an independent worker and clears values when the dialog closes. |
| Network | Captures TCP/UDP endpoints for the fixed live process through the shared Network worker on first activation or explicit refresh. |
| Scheduling | Reads the fixed live process's CPU priority class on its own worker, with explicit confirmed changes and restoration. |

Image, Files, DLL, Environment, Network, and Scheduling collection never runs as part of ordinary sampling. Each request carries the dialog generation; collectors can also carry request IDs to distinguish superseded work. Results from a closed, reopened, or superseded dialog are rejected even if they refer to the same PID.

Image collection may inspect loaded `coreclr.dll` or `clr.dll` to report the active .NET runtime version. This does not add module enumeration to normal sampling or Recording.

## Open Files and DLLs

Open Files lists disk files currently open by the fixed live process. It is not a general handle browser for pipes, sockets, registry keys, synchronization objects, or every Windows handle type.

Each named file handle occupies its own entry, even when several handles refer to the same path. Cache configuration, write-through, synchronous/asynchronous-capable mode, and data access rights are shown independently for that handle. There is no path-level aggregation or `Mixed` value. Details retain the full path, original process handle value, raw access/mode masks, and per-field query failures.

The collector duplicates each original handle with the same access rights and queries that duplicate; it never reopens a pathname to infer the original opening mode. Unavailable attributes do not hide an otherwise identified file handle or replace its other known attributes. These values describe the captured opening configuration, not cache hits or observed I/O completion behavior. They remain outside sampling, Recording, exports, and configuration. Log view never starts this collection.

Files content accepts filter text directly. Selection and details operate on individual handles. Refresh retains selection by path and process-local handle value when present; that value can be reused after a close and is not a persistent identity across captures. A disappeared selected handle returns the view to the list. The list and detail view share responsive drawing and mouse geometry; details preserve complete values on narrow terminals. Clipboard output copies full handle rows for the filtered list, or the selected handle in details.

DLL collection is an explicit point-in-time Toolhelp snapshot. File metadata failures remain per-row unavailable values rather than failing the whole list. Files and DLL filters search complete displayed paths, and explicit refresh must not queue redundant work for the same dialog session.

Files schedules its next automatic refresh after the previous result completes. The idle interval is at least two seconds and at least ten times the measured collection duration, rounded up to whole seconds. Collection duration includes both identity checks and handle/attribute enumeration. A request taking more than one second, or a failed request, stops automatic refresh; an inexpensive successful manual refresh resumes it. The tab shows the interval and last collection duration, or why automatic refresh stopped. No collection is added to normal sampling.

Only the active Files tab schedules automatic requests. Switching tabs, closing Process Info, entering Log view, or losing the fixed live process stops scheduling. An in-flight request can finish, but closed or replaced sessions reject its result. There is no catch-up queue after returning to Files. Filters and selected handles survive automatic refresh under the same rules as manual refresh. Automatic requests do not repeatedly overwrite unrelated status messages. Global Find file users searches remain explicit.

Both collectors run outside the UI and sampling threads. Process identity is checked before results are accepted. Files checks only the target process's name and creation time before and after capture, without collecting unrelated host metrics or process metadata. This refresh policy controls request frequency, not the duration of a native filesystem call already in progress.

## Find File Users

Find file users searches disk-file handles across the host, independently of the Processes filter, selection, and Tracked-only setting. Opening the browser or editing the query performs no scan. Only an explicit search or repeat starts collection. Live, display pause, and Recording allow this investigation; Log view does not.

Each capture acquires the system handle table once and shares handle duplication and path-resolution primitives with Files. Source process handles are acquired before the table and their native creation times are checked before and after inspection. Results group duplicate handles by captured process lifetime and matched path, showing a handle count without combining I/O attributes.

Potentially blocking native calls run in a hidden helper process belonging to a job with kill-on-close and a memory limit. A controller thread reads bounded progress messages and publishes only its latest cumulative result. Cancellation, inactivity, overall deadlines, and result limits terminate the scan while retaining confirmed matches. Shutdown requests helper termination and uses a bounded wait. If Windows has not completed termination, the result explicitly reports pending cleanup; a faulty filesystem driver can delay kernel cleanup beyond application control. The helper never terminates investigated processes.

Results distinguish completed, cancelled, timed-out, limited, and failed captures. Process-level access failures and exits, handle-level failures, unnamed disk-file handles, and unvisited handles have separate counts. A zero-result capture says no matches were found in the inspected scope, without claiming a file has no users or that a matching handle necessarily prevents deletion. Details retain full paths and coverage information when the table is clipped.

Navigating reopens and verifies the owner's native creation time in the helper before opening its Files tab. Closing Process Info returns to retained search results. Request IDs reject replies after cancellation, a new search, or a closed browser. Query drafts and the query associated with captured results remain distinct. Queries, results, and helper protocol data are not saved to configuration, sampling history, Recording, or exports.

Matching semantics, limits, and clipboard fields are owned by [metrics.md](metrics.md). Memory-mapped-only file use, hard links, short names, reparse aliases, and uninspectable processes are outside an exhaustive ownership guarantee.

## Environment

Environment is a best-effort Windows 11 x64 investigation action. The worker handles native x64 and WOW64 pointer widths, validates remote-memory regions, enforces a 4 MiB limit, and requires valid terminated UTF-16 data.

Environment values may contain passwords, tokens, or other secrets. They remain in dialog-owned memory, are cleared when Process Info closes, and never enter status text, error text, Recording, exported data, or Log view.

## Log View and A/B Data

Log view never starts live Image, Files, DLL, Environment, or Network workers. Metrics and recorded Image fields use loaded data when present; dynamic tabs show that their data was not recorded.

Process Info comparisons resolve A, B, and displayed-current values by exact `ProcessIdentity` and exact `captured_at`. Nearby samples, the latest Ghost Row value, and samples from a reused PID are not substituted. A delta is calculated only when both exact values exist.

## Network Endpoints

The global Network browser and Process Info Network tab share the same collector, endpoint model, filter, table, detail view, and clipboard format. The global browser defaults to TCP listeners plus bound UDP endpoints. The process tab defaults to all available endpoints for the dialog's fixed target, including connected TCP peers. Both can change the local display mode without collecting again.

The Process Info Network tab follows the other filterable tabs: printable input edits the filter directly while content has focus, without entering a separate editing mode. Row navigation, refresh, opening details, and closing the dialog remain available during filtering. The global browser retains its explicit filter-editing mode. Contextual keys are defined in Help and the respective footers.

Opening the browser or first activating the tab requests one capture. Refresh is explicit, stays on the independent Network worker, and does not queue another request for the same pending session. The worker has a bounded request queue. Tab switches retain the capture, filter, and selection; closing a session invalidates its pending work. Every result must match its target, generation, and request ID. Selection survives refresh by endpoint key where possible.

The four protocol/family tables are captured separately, so the report describes a capture interval rather than an atomic system snapshot. Successful tables remain visible if another table fails. The view shows capture time, successful-table coverage, and unresolved-owner counts. Full capture errors are available in details, including when no endpoint is selected. A failed refresh preserves the previous capture with its original timestamp and a visible notice.

Owner verification brackets table capture with a held process handle and native creation time. Rows remain visible with an unavailable owner when verification fails. Opening Process Info from a global row performs a fresh creation-time check on the worker; an exited or replaced process cannot be opened by reusing its PID. The Process Info target does not depend on the current Processes selection or filter. Closing Process Info returns to the retained global results.

Network investigation remains available during Live, display pause, and Recording. Endpoint reports are session-local and never enter samples, histories, configuration, Recording, or exports. Existing metric recording continues independently. Log view has no global Network browser and the process tab displays its not-recorded state. There is no DNS lookup, traffic capture, periodic endpoint polling, or endpoint modification.

## Scheduling

Scheduling captures the CPU priority class on first activation or explicit refresh. Its worker retains a process handle after verifying the executable name and creation time against the fixed dialog identity. Subsequent reads and writes use that same process object and check that it is still alive; they do not reopen a PID for each operation. Read access without change permission remains useful and is shown as read-only. Closing the dialog invalidates queued work and releases the handle; reopened sessions reject old results.

The editor offers Idle, Below normal, Normal, Above normal, and High. An existing Realtime or unknown class is displayed but cannot be selected or restored. Changing a class requires reviewing the current and proposed values and then explicitly confirming. While application is pending, the dialog waits for the result before accepting navigation or another action. Display pause and Log view disable changes; Log view does not start a live Scheduling request. Recording may continue independently, without recording the setting or action.

Immediately before changing priority, the worker re-reads the class and rejects a change if it differs from the value confirmed by the user. It reads back after a successful API call and distinguishes verified success, failure before application, and an accepted change whose readback failed or differs. Windows does not provide an atomic compare-and-set operation for priority classes; another actor can still race the short read/write interval. The tool does not impose an ongoing policy over other actors.

Each accepted change retains one previous class and the class written by this session. Explicit restoration requires that the observed current class still equals the class this session wrote, and uses the same review, permission, lifetime, and readback checks. Restoration does not overwrite a detected external change. Refreshing and switching tabs retain this restoration point; closing the dialog discards it. Nothing restores automatically on close or application exit, and no priority policy is saved in configuration or Investigation Profiles. CPU quota, I/O priority, Efficiency mode, and affinity are outside this editor's scope.

## Input and Layout Boundaries

The dialog's tab, content, detail, and scrollbar hit regions are derived from the same responsive layout used for drawing. Clicking outside the modal neither dismisses it nor operates underlying panels.

Passive tabs keep navigation on their content without creating a false focus stop. Interactive tabs separate tab selection from selectable or filterable content. Exact keys and footer guidance remain owned by the in-app Help, dialog implementation, and rendering tests.

## Invariants

- Every tab uses the process identity fixed when the dialog opened.
- Asynchronous results must match both target identity and dialog generation.
- Blocking process-specific collection never runs on the UI or sampling thread.
- Log view never starts live process-investigation workers.
- Environment values never leave dialog-owned state or appear in diagnostic text.
- Process comparisons never substitute a nearby time or different identity.
- Drawing and hit testing use the same dialog geometry.
