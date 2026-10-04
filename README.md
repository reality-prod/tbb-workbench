# TBB Workbench

**A desktop workbench for Tor Browser's Reproducible Build Manager (RBM).**

TBB Workbench is a local GUI for inspecting, configuring, and running an existing [`tor-browser-build`](https://gitlab.torproject.org/tpo/applications/tor-browser-build) / [RBM](https://gitlab.torproject.org/tpo/core/rbm) checkout.

Instead of replacing RBM or hiding the build process behind a custom abstraction, TBB Workbench provides a graphical layer around the tools and configuration that already exist in the repository.

> **Project status:** Early-stage, functional, and actively evolving. macOS is currently the only packaged platform that has been tested. Linux and Windows packaging are planned.
>
> **AI disclosure:** This project is currently AI-generated. The repository is intentionally published as Free and Open Source Software so the implementation can be inspected, tested, reviewed, corrected, and improved openly.
>
> **Tor affiliation:** TBB Workbench is an independent project and is **not affiliated with, endorsed by, or produced by The Tor Project**.

---

## Why TBB Workbench?

Working with RBM can involve a lot of moving parts: Git state, configuration files, build targets, host dependencies, long-running subprocesses, logs, artifacts, and environment diagnostics.

TBB Workbench puts those pieces into one desktop application while keeping the underlying workflow visible.

It is intended to make questions like these easier to answer:

* What state is my checkout in?
* What build targets are actually available?
* What does this RBM configuration contain?
* Is my machine missing a required dependency?
* What is the build doing right now?
* Why did the build fail?
* What artifacts were produced?
* What changed in the repository?

The application is a **workbench for RBM**, not a replacement for RBM.

---

## Features

### Repository

Open and inspect an existing Git checkout with:

* Branch and commit information
* Ahead/behind status
* Dirty/clean state
* Per-file status
* Tags
* Submodules
* Remotes with credentials redacted

Git operations such as clone, fetch, checkout, submodule update, and diff are constructed as argument-based commands and executed through the same supervised process system used for builds.

### Build

Run the repository's real build workflow from a graphical interface.

Features include:

* Makefile target discovery
* Build presets
* Custom/advanced argument mode
* Environment variable overrides
* Preflight checks
* Live stdout/stderr output
* Warning and error detection
* Build phase detection
* Heuristic progress estimation
* Build cancellation
* Persistent raw logs
* Artifact discovery

Makefile targets are discovered by parsing the Makefile. The application does not execute build targets simply to discover what commands are available.

### RBM configuration

Inspect and edit repository files without leaving the application.

The editor includes:

* Repository-aware file tree
* `.gitignore` awareness
* YAML validation
* YAML, JSON, Markdown, Shell, and Perl syntax highlighting
* CodeMirror 6 editing
* Unsaved-change indicators
* Save, revert, and save-all
* External-change conflict detection
* Diff against disk
* Diff against `HEAD`

The RBM inspector can identify, where the input is parseable:

* Variables
* Targets
* Input files
* Includes

When a file cannot be reliably interpreted as plain YAML, the inspector reports that rather than inventing an answer.

### Diagnostics

Inspect the local environment and export useful diagnostic information, including:

* Operating system
* CPU
* Memory
* Disk space
* Git version
* Make version
* Python 3
* Perl
* CPAN
* Podman/Docker
* Linux user-namespace tooling
* Repository health

Diagnostics can be exported in text or JSON form.

### Logs

Build and run history is persisted locally.

The log viewer supports:

* Full raw logs
* Filtering
* Wrapping
* Export
* Deletion
* Retention policies
* Redacted run metadata

Runs that are interrupted because the application exits are recorded explicitly rather than being presented as successful or silently reattached later.

### Artifacts

Expected output directories can be scanned after builds.

Optional SHA-256 hashing is available, and generated artifacts can be revealed in the system file manager.

### Dependency setup

The application can detect required build tooling and currently handles installation of required Perl modules and CPAN tooling as part of the supported workflow.

---

## Platform status

| Platform | Application | Packaging      | Status      |
| -------- | ----------- | -------------- | ----------- |
| macOS    | ✅           | ✅ Tauri bundle | Tested      |
| Linux    | ⚠️          | 🚧 Planned     | Coming soon |
| Windows  | ⚠️          | 🚧 Planned     | Coming soon |

The current build configuration in `Makefile.toml` is intentionally focused on producing the **macOS Tauri bundle**.

Linux and Windows build targets will be added as their respective workflows are implemented and tested.

### Unix-oriented workflow

TBB Workbench is primarily designed around a **Unix-like development and build environment**. That reflects the ecosystem it is built for: RBM, `tor-browser-build`, Git, Make, Perl, Python, containers, and related build tooling.

Windows support is planned at the application/package level, but the underlying RBM workflow may still require a Unix-like environment depending on the repository and build configuration being used.

---

# Safety and process model

TBB Workbench interacts with repositories, files, and external build tools, so it is designed to make those actions explicit.

### Explicit operations

There is no hidden "do everything" workflow.

Repository operations such as clone, fetch, checkout, and submodule update are user-triggered actions.

The application does not expose destructive Git commands such as:

```text
git reset --hard
git clean -fdx
git rebase
git pull
```

### Supervised processes

Builds and Git operations run as real subprocesses through a common process supervisor.

Normal execution uses argv-based commands rather than shell command strings.

Shell execution is an explicit advanced feature and is disabled by default.

To use it, the user must enable:

```text
Settings → Safety → Allow shell command mode
```

### Filesystem protection

Repository file access is guarded against:

* `..` path traversal
* Symlink escapes
* Writing outside the permitted repository path

### Credential and environment redaction

Environment values and repository remotes are filtered for likely secrets before being written to logs, previews, or diagnostic bundles.

Redaction is heuristic, not a cryptographic guarantee. Users should still review diagnostic output before publishing it.

### Network behavior

TBB Workbench does not include telemetry or a background cloud service.

Network access comes from operations explicitly initiated by the user or from tools involved in the build workflow, such as Git or container tooling.

---

# Architecture

The project is split into a reusable Rust core and a Tauri desktop layer.

```text
TBB Workbench
│
├── crates/
│   └── tbb-core/
│       ├── Repository inspection
│       ├── Git operations
│       ├── Process supervision
│       ├── Build execution
│       ├── Filesystem guards
│       ├── RBM inspection
│       ├── Makefile parsing
│       ├── Diagnostics
│       ├── Run history
│       └── Artifact scanning
│
├── src-tauri/
│   └── Tauri commands and desktop integration
│
└── frontend/
    ├── React
    ├── TypeScript
    ├── Vite
    └── CodeMirror 6
```

`tbb-core` is intentionally independent of Tauri so that the majority of the application's core behavior can be tested without launching the desktop UI.

---

# Design

The interface takes inspiration from publicly documented **Tor Marble** design guidance.

This includes ideas such as:

* Typography scale
* Spacing
* Component states
* Iconography
* General UI conventions

The application uses its own implementation of these ideas and does **not** use Tor Project logo assets.

The UI uses:

* Space Grotesk / Inter-inspired typography
* Phosphor icons
* Bootstrap-style component states
* A small shared component system for buttons, cards, badges, alerts, modals, and tabs

**Design inspiration does not imply affiliation or endorsement.**

---

# Getting started

## Requirements

Development currently requires:

* Rust 1.80+
* Node.js and npm
* Tauri platform prerequisites
* Git
* Make
* Python 3
* Perl
* CPAN
* The dependencies required by the RBM / `tor-browser-build` repository you intend to build

Install Rust with [rustup](https://rustup.rs/).

See the [Tauri prerequisites](https://tauri.app/start/prerequisites/) documentation for platform-specific dependencies.

### macOS

Install Xcode Command Line Tools:

```bash
xcode-select --install
```

Linux and Windows build prerequisites will be documented as their platform targets are added.

---

## Build the application

Install JavaScript dependencies:

```bash
npm install
```

The project uses [Cargo Make](https://github.com/sagiegurari/cargo-make) for its application build workflow.

Run the development application:

```bash
cargo make dev
```

Build the current macOS Tauri bundle:

```bash
cargo make build
```

The available build targets are defined in `Makefile.toml`.

---

## Frontend development

For a TypeScript check:

```bash
npm run typecheck
```

For a frontend production build:

```bash
npm run build
```

These commands do not require the Rust/Tauri application to be running.

---

## Rust tests

The core crate can be tested independently:

```bash
cd crates/tbb-core
cargo test
```

Run Clippy with:

```bash
cargo clippy -p tbb-core --all-targets
```

---

# Verification

The core project has been verified with real tooling rather than mock command output.

Current checks include:

```bash
cargo test -p tbb-core
cargo clippy -p tbb-core --all-targets
npm install
npx tsc -b --noEmit
npx vite build
```

The frontend build produces a real `dist/` directory.

The core Rust test suite currently includes integration coverage for subprocess cancellation and forceful termination of real long-running processes.

The Tauri application has been tested on macOS.

The Tauri command layer itself still needs additional dedicated integration coverage.

---

# Development notes

## Tauri version matching

The Rust and JavaScript Tauri packages are pinned to matching versions.

The following package families must remain synchronized:

| Rust                  | JavaScript                  |
| --------------------- | --------------------------- |
| `tauri`               | `@tauri-apps/api`           |
| `tauri-plugin-dialog` | `@tauri-apps/plugin-dialog` |
| `tauri-plugin-fs`     | `@tauri-apps/plugin-fs`     |
| `tauri-plugin-opener` | `@tauri-apps/plugin-opener` |

Current pinned versions:

| Package family |  Version |
| -------------- | -------: |
| Tauri          | `2.12.0` |
| Dialog         |  `2.8.0` |
| Filesystem     |  `2.6.0` |
| Opener         |  `2.7.0` |

Do not update one side independently. Rust and JavaScript package versions should be changed together in the same commit.

---

# Known limitations

This is not a finished product.

Current limitations include:

* Only the macOS packaged application has been tested
* Linux packaging is not yet implemented
* Windows packaging is not yet implemented
* Frontend component tests are not yet implemented
* End-to-end tests are not yet implemented
* CI workflows are not yet included
* The command palette UI is not wired up yet
* Builds interrupted by application exit are recorded as interrupted and are not automatically reattached after restart
* The Tauri command layer needs more dedicated integration testing

These are known gaps, not hidden functionality.

---

# AI-generated software

TBB Workbench is currently an **AI-generated project**.

Generative AI has been used substantially throughout the implementation. This is disclosed deliberately.

The project is published openly because the goal is not to present AI-generated code as something it is not. The repository is meant to be examined by humans, challenged, tested, improved, and maintained in the open.

That also means the usual software rules still apply:

**Review the code. Test the code. Assume bugs exist.**

The fact that code was generated by an AI system is neither a security guarantee nor a security failure by itself. What matters is what the code actually does.

---

# Contributing

Contributions and review are welcome.

Useful contributions include:

* Linux testing
* Windows testing
* Build-system improvements
* Tauri integration tests
* Frontend tests
* Security review
* RBM compatibility testing
* Documentation improvements
* Bug fixes

When reporting an issue, include enough information to reproduce it where possible, such as your operating system, application version/commit, tool versions, RBM checkout, and relevant logs.

Remove credentials and other sensitive information before posting logs publicly.

---

# License

TBB Workbench is Free and Open Source Software.

See [`LICENSE`](LICENSE) for the license applicable to this repository.

---

# Disclaimer and trademark notice

TBB Workbench is an independent project.

It is **not affiliated with, endorsed by, sponsored by, or produced by The Tor Project**.

Tor and Tor Browser are trademarks of The Tor Project.

TBB Workbench does not use Tor Project logo assets and does not present itself as an official Tor Project application.

The project's interface is informed by publicly documented Marble design guidance. This design inspiration does not constitute endorsement, partnership, or affiliation.
