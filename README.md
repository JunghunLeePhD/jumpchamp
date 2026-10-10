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
│   │   ├── mod.rs               # Re-exports wheel, parallel, stream
│   │   ├── wheel.rs             # small_primes, sieve_segment (bitpacked wheel-of-30 sieve)
│   │   ├── parallel.rs          # sieve_range_parallel (Rayon L1-cache bitmask dispatcher)
│   │   └── stream.rs            # prime_blocks_up_to, stream_prime_blocks_range, nth_prime_upper_bound
│   ├── analysis/
│   │   ├── mod.rs               # Re-exports gaps, report
│   │   ├── gaps.rs              # apply_interval, k_step_gaps, record_gaps, count_residues
│   │   └── report.rs            # format_report, format_record_gaps_report, format_residue_report
│   ├── engine/
│   │   ├── mod.rs               # Pure GUI-free computation engine root
│   │   ├── histogram.rs         # Pure histogram operations and top_gaps sorting
│   │   ├── scan.rs              # Unified prime walk with progress and cancellation
│   │   ├── range.rs             # Range histogram queries and ChunkCache
│   │   └── frames.rs            # FrameSet precomputation for animations
│   ├── gui/
│   │   ├── mod.rs               # GUI domain root
│   │   ├── actions.rs           # Pure action dispatcher and event loop
│   │   ├── app.rs               # eframe::App window shell & update loop
│   │   ├── format.rs            # Formatting utilities (compact, thousands)
│   │   ├── playback.rs          # Pure playback state machine
│   │   ├── prefs.rs             # User preferences (theme, limits, toggles)
│   │   ├── state.rs             # AppState, Query, Rank
│   │   ├── theme.rs             # Unified Palette definition & Viridis colormap
│   │   ├── worker.rs            # Non-blocking background worker with epoch-based cancellation
│   │   ├── widgets/             # Reusable UI widgets (dual-thumb range slider, index input)
│   │   └── panels/              # Composable UI panels (chart, playback_bar, settings, status_bar, toolbar)
│   └── bin/
│       └── jumpchamp_gui.rs     # Native desktop GUI entry point binary
├── .gitignore                   # Ignores /target build artifacts
└── Cargo.toml                   # Dependencies (Rayon, egui, eframe, crossbeam-channel)
```

Each domain layer is independently readable and testable:

| Layer | Modules | Responsibility | External Deps |
|---|---|---|---|
| `sieve/` | `wheel`, `parallel`, `stream` | Bitpacked wheel-of-30 prime generation & bounds | `rayon` |
| `analysis/` | `gaps`, `report` | Pure gap combinators, Cramér ratios & text reports | none |
| `engine/` | `histogram`, `scan`, `range`, `frames` | Gap histograms, chunk caching & animation frames | `sieve`, `analysis` |
| `gui/` | `state`, `worker`, `actions`, `panels`, `widgets` | Native desktop interface & virtualized rendering | `egui`, `eframe`, `egui_plot`, `crossbeam-channel` |

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

You can package the double-clickable `JumpChamp.app` bundle locally with native icons:

#### Option 1: Automated Script (Recommended)
Run the automated packaging script (automatically checks prerequisites, installs `cargo-bundle` if missing, packages the app, and signs it):

```bash
./scripts/package_macos_app.sh
```

#### Option 2: Manual Commands
```bash
# 1. Install cargo-bundle plugin and ensure ~/.cargo/bin is in your PATH
cargo install cargo-bundle
export PATH="$HOME/.cargo/bin:$PATH"

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

## 👨‍💻 For Developers: Managing Homebrew Distribution

JumpChamp is distributed to macOS users via a custom Homebrew Tap ([`JunghunLeePhD/homebrew-tap`](https://github.com/JunghunLeePhD/homebrew-tap)).

### 🔄 End-to-End Release & Tap Update Workflow

When publishing a new release (e.g. `v1.0.1`):

1. **Bump Version in `Cargo.toml` & Merge**:
   - Update `version = "1.0.1"` in `Cargo.toml` (package and bundle metadata).
   - Merge into `main` and push:
     ```bash
     git push origin main
     ```

2. **Tag & Trigger GitHub Actions**:
   - Create and push the release tag:
     ```bash
     git tag v1.0.1
     git push origin v1.0.1
     ```
   - Monitor GitHub Actions at [`.github/workflows/release-gui.yml`](file:///workspace/.github/workflows/release-gui.yml).
   - Wait 2–3 minutes for both Apple Silicon and Intel macOS runners to build and upload the release `.app.zip` assets.

3. **Generate Cask Definition with Checksums**:
   - Once the release assets are published, run the automated generator script:
     ```bash
     ./scripts/generate_brew_cask.sh v1.0.1
     ```
   - This downloads the release assets, computes the verified SHA-256 checksums, and updates [`packaging/homebrew/Casks/jumpchamp.rb`](file:///workspace/packaging/homebrew/Casks/jumpchamp.rb).
   - Commit the updated Cask definition:
     ```bash
     git add packaging/homebrew/Casks/jumpchamp.rb
     git commit -m "feat(brew): update cask definition for v1.0.1 release"
     git push origin main
     ```

4. **Update `homebrew-tap` Repository**:
   - Copy `packaging/homebrew/Casks/jumpchamp.rb` to `Casks/jumpchamp.rb` in your [`JunghunLeePhD/homebrew-tap`](https://github.com/JunghunLeePhD/homebrew-tap) repository.
   - Commit and push to `main` on `homebrew-tap`:
     ```bash
     git add Casks/jumpchamp.rb
     git commit -m "feat: bump jumpchamp cask to v1.0.1"
     git push origin main
     ```

5. **Upgrade & Verify on macOS**:
   ```bash
   brew update
   brew upgrade --cask jumpchamp
   ```

### ⚠️ Troubleshooting: Checksum Mismatch Error
If `brew upgrade` reports `Error: Cask reports different checksum`:
- **Cause**: The Cask generator script was executed before GitHub Actions finished uploading the zip archives, causing it to hash GitHub's 404 "Not Found" response (`0019dfc4...`).
- **Fix**: Re-run `./scripts/generate_brew_cask.sh vX.Y.Z` after the release assets appear on the GitHub Release page, and push the updated `Casks/jumpchamp.rb` to `homebrew-tap`.

---

## 📜 License

MIT License. Feel free to use and modify for analytical and educational research.
