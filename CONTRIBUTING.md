# Contributing to winproc-tui

`winproc-tui` is a personal project. Do not open a pull request unless the maintainer has explicitly asked you to submit that specific change.

## Bugs, Feedback, and Feature Requests

Use [GitHub Issues](https://github.com/TX230/winproc-tui/issues) to report a bug, provide feedback, or propose a feature. Issues may be written in English or Japanese.

Issues are for reports and discussion only. Opening an Issue, receiving maintainer feedback, or reaching agreement on a proposal does not grant permission to submit a pull request.

Before opening an Issue:

1. Search existing open and closed Issues for the same topic.
2. Use the Bug report or Feature request template.
3. Keep one problem or proposal per Issue.
4. Remove secrets and personal information from screenshots and logs.

Report suspected vulnerabilities privately as described in [SECURITY.md](SECURITY.md), not in a public Issue.

## Development Issue Policy

Use a GitHub Issue to track implementation work when it:

- adds or changes a user-facing feature;
- corrects a user-visible bug;
- changes a specification, compatibility guarantee, or development-workflow policy; or
- needs investigation, discussion, acceptance criteria, or progress tracking.

An Issue is not required when the work is limited to:

- version updates and routine release preparation;
- typo, formatting, or comment corrections;
- documentation corrections that only align text with existing behavior and policy;
- behavior-preserving refactoring or warning cleanup; or
- routine dependency, CI, or packaging maintenance that does not change user-facing behavior, specifications, or compatibility.

Commit count, file count, and implementation size do not determine whether an Issue is required. A small feature or bug fix still requires an Issue. If an existing Issue already covers the work, use it instead of creating a duplicate. If initially Issue-free work expands into a category that requires an Issue, create or select the Issue before continuing with that expanded scope.

## Maintainer-Requested Pull Requests

If the maintainer explicitly asks you to submit a pull request for a specific change:

- keep the change narrowly scoped to the request;
- follow the ownership map in [docs/architecture.md](docs/architecture.md);
- keep user-facing English and Japanese README changes synchronized;
- run `cargo fmt --all -- --check` and `cargo test` when Rust code changes;
- use an English Conventional Commit with a concise explanatory body;
- complete the repository pull request template accurately.

AI coding agents must also follow [AGENTS.md](AGENTS.md).
