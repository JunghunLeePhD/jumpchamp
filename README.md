# Jump Champ 🦀📊

A high-performance, native desktop application in Rust (`egui` + `eframe`) for interactive prime gap distribution exploration, live in-memory segmented sieving, and GPU-accelerated visualizations.

Built with a functional programming architecture, cache-aligned multi-threading via **Rayon**, and **VS Code Dev Containers** support.

---

## 🌟 Key Features

* **Native Desktop GUI (`egui` + `eframe`)**: Cross-platform desktop interface featuring non-blocking in-memory computation, GPU-accelerated plots (`egui_plot`), virtualized table inspection (`egui_extras`), and real-time LTTB data downsampling for smooth 60+ FPS rendering of datasets with millions of primes.

* **Parallel Bitpacked Segmented Sieve**: Utilizes Rayon to process 64-bit bitmasked odd-only segments in parallel, fitting 262,144 candidates into CPU L1-cache for an **8x-16x memory footprint reduction**.

* **Non-blocking Background Worker**: Sieve calculations and gap frequency computations execute on dedicated worker threads, ensuring continuous 60 FPS UI responsiveness with live progress reporting.

* **Advanced Mathematical Analytics**:
  * **Record Gap Tracking**: Computes maximal record-breaking prime gaps $\Delta(n)$ and Cramér Ratios $C(n) = \frac{\Delta(n)}{(\ln p_n)^2}$.
  * **Residue Class Analysis**: Analyzes prime gap modulo alignments ($g \pmod 6$, $g \pmod{30}$).
  * **Markov Gap Transitions**: Analyzes 2-step gap transition probabilities $(g_n \to g_{n+1})$.

* **Comprehensive Test Suite**: Pure functional architecture with unit tests for algorithms, iterator combinators, LTTB downsampling, and UI state management (`cargo test`).

---

## 📁 Project Structure

```text
jumpchamp/
├── .devcontainer/
│   └── devcontainer.json        # VS Code Dev Container settings
├── src/
│   ├── lib.rs                   # Library root — declares all domain layers
│   ├── main.rs                  # Primary GUI application entry point
│   ├── sieve/
│   │   ├── mod.rs               # Re-exports basic, parallel, stream
│   │   ├── basic.rs             # small_primes, sieve_segment (bitpacked odd-only sieve)
│   │   ├── parallel.rs          # sieve_range_parallel (Rayon L1-cache bitmask dispatcher)
│   │   └── stream.rs            # stream_prime_blocks_range (lazy block iterator)
│   ├── analysis/
│   │   ├── mod.rs               # Re-exports gaps, report
│   │   ├── gaps.rs              # apply_interval, k_step_gaps, record_gaps, count_residues
│   │   └── report.rs            # format_report, format_record_gaps_report, format_residue_report
│   ├── gui/
│   │   ├── mod.rs               # GUI domain root
│   │   ├── animation.rs         # Animation state transitions & step dispatching
│   │   ├── app.rs               # eframe::App window shell & update loop
│   │   ├── state.rs             # AppState, WorkerCommand, WorkerResult
│   │   ├── theme.rs             # Viridis dark & light theme palettes
│   │   ├── utils.rs             # Formatting utilities (numbers, thousands)
│   │   ├── worker/
│   │   │   ├── mod.rs           # Worker module root (re-exports spawn_worker)
│   │   │   ├── dispatch.rs      # Non-blocking background worker thread & command loop
│   │   │   └── engine.rs        # Sieve math, segment histogram caching & bounds calculation
│   │   └── panels/
│   │       ├── chart.rs         # Interactive egui_plot normalized histogram & heatmap meter
│   │       ├── settings.rs      # Modal settings & theme preferences window
│   │       ├── status_bar.rs    # Bottom telemetry status bar component
│   │       └── sidebar/         # Top dual-thumb range sliders & toolbars
│   └── bin/
│       └── jumpchamp_gui.rs     # Native desktop GUI entry point binary
├── .gitignore                   # Ignores /target build artifacts
└── Cargo.toml                   # Dependencies (Rayon, egui, eframe, crossbeam-channel)
```

Each domain layer is independently readable and testable:

| Layer | Modules | Responsibility | External Deps |
|---|---|---|---|
| `sieve/` | `basic`, `parallel`, `stream` | Bitpacked odd-only prime generation | `rayon` |
| `analysis/` | `gaps`, `report` | Gap analysis, Cramér ratios, residues & report formatting | none |
| `gui/` | `app`, `state`, `worker`, `theme`, `panels` | Native desktop interface & virtualized rendering | `egui`, `eframe`, `egui_plot`, `crossbeam-channel` |

---

## 🚀 Quick Start

### Prerequisites
- **Rust** (1.70+ recommended) OR **VS Code with Dev Containers** extension.

### 1. Run from Source

Launch the native desktop GUI application with a single command:

```bash
cargo run --release
```

Or specifically via the binary target:

```bash
cargo run --release --bin jumpchamp_gui
```

### 2. macOS Installation via Homebrew

Install the standalone macOS desktop app:

```bash
brew install --cask JunghunLeePhD/tap/jumpchamp
```

> [!TIP]
> The Homebrew Cask includes an automated `postflight` hook that removes macOS quarantine flags upon installation, allowing JumpChamp to launch immediately without requiring manual security bypass commands.

---

## 🖥️ Key GUI Capabilities

- **3-Panel Workflow**: Sidebar controls for $k$-step gap size and prime search range $[n, m]$, interactive `egui_plot` visualizer, and virtualized histogram bars.
- **In-Memory Sieve Engine**: Computes primes on the fly using L1-cache aligned segmented sieve without requiring pre-computed database files on disk.
- **LTTB Downsampling**: Dynamically reduces dense data series down to ~2,000 display points for real-time panning/zooming at 60+ FPS.
- **Non-blocking Execution**: Background worker channel prevents UI freezes while generating and analyzing prime ranges.

---

## 🛠️ Architecture & Functional Design

The project strictly follows Functional Programming (FP) principles:

- **Pure Functions**: `small_primes` and `sieve_segment` are pure and free of side-effects, making them easily unit-testable without filesystem access.
- **Lazy Evaluation**: `stream_prime_blocks_range` streams blocks lazily, keeping memory bounded regardless of total primes generated.
- **Decoupled Worker Architecture**: The GUI main thread handles event loop rendering at 60 FPS while the computation worker executes in a separate thread communicating over crossbeam channels.

---

## 📦 Packaging & Distributing Standalone Executables

### Building macOS Application Bundles Locally

To compile and package the double-clickable `JumpChamp.app` bundle locally on macOS:

```bash
# 1. Install cargo-bundle
cargo install cargo-bundle

# 2. Package .app bundle
cargo bundle --release --bin jumpchamp_gui

# 3. Ad-hoc codesign the bundle (required for Apple Silicon)
codesign --force --deep --sign - target/release/bundle/osx/JumpChamp.app
```

The resulting `JumpChamp.app` will appear in:
`target/release/bundle/osx/JumpChamp.app`

---

## 🤖 Automated GitHub Actions GUI Releases (macOS)

This repository features an automated GitHub Actions CI/CD pipeline ([`.github/workflows/release-gui.yml`](file:///workspace/.github/workflows/release-gui.yml)) targeting Apple Silicon and Intel Macs.

Whenever a version tag is pushed (e.g. `v1.0.0`) or dispatched manually:
1. GitHub Actions spins up native macOS runners for both **Apple Silicon (`aarch64`)** and **Intel (`x86_64`)**.
2. Packages and ad-hoc signs `JumpChamp.app`.
3. Attaches `JumpChamp-macos-arm64.app.zip` and `JumpChamp-macos-intel-x86_64.app.zip` directly to the GitHub Release.

---

## 📜 License

MIT License. Feel free to use and modify for analytical and educational research.
