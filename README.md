# Jump Champ 🦀📊

A high-performance, native desktop application in Rust (`egui` + `eframe`) for interactive prime gap distribution exploration, live in-memory segmented sieving, and GPU-accelerated visualizations.

Built with a functional programming architecture, cache-aligned multi-threading via **Rayon**, and **VS Code Dev Containers** support.

---

## 🌟 Key Features

* **Native Desktop GUI (`egui` + `eframe`)**: Cross-platform desktop interface featuring non-blocking computation, GPU-accelerated bar plots (`egui_plot`), and a dynamic 2D `egui::ColorImage` spectral distribution heatmap across prime-index scales.

* **Zero-Copy Binary Storage & Two-Tier Indexing (`memmap2` + `bytemuck`)**:
  * **Instant Startup (<1 ms)**: Memory-maps pre-aggregated 2.4 KB `BlockHistogram` tables (`block_index.bin`, ~9 MB up to $10^{12}$) with zero parsing or decompression.
  * **Sub-Millisecond Range Queries**: `DistributionEngine::query_distribution` sums spanning block histograms ($L = 10^7$) in microseconds across ranges up to $37.6 \times 10^9$ primes ($\pi(10^{12})$).
  * **On-the-Fly $k$-Step Gaps**: Memory-maps raw `u16` 1-step gaps (`gaps.bin`, ~75 GB at $10^{12}$) and evaluates higher-order $k$-step gaps dynamically via sliding-window sums over active viewports without storing duplicate databases.

* **Parallel Bitpacked Segmented Sieve Fallback**: Uses Rayon over 64-bit wheel-of-30 segments fitting 262,144 candidates in L1 cache for live out-of-the-box exploration without pre-generated binary files.

* **Non-blocking Background Worker & Instant Reset**: Background worker threads communicate over `crossbeam-channel` with epoch-synchronized `CancelToken` polling at sieve segment, engine scan, and mmap query boundaries, keeping the UI locked at 60 FPS and allowing instant cancellation mid-computation.

* **Advanced Mathematical Analytics**:
  * **Top-5 Gap Ranking Board**: Floating HUD ranking the top 5 most frequent gaps (`#1`–`#5`) with live counts, percentages, and interactive bar focus lock.
  * **Prime Factorization & Primorial ($p_n\#$) Alignment**: Computes exact prime factorizations ($\Delta = \prod p_i^{a_i}$), detects primorial multiples ($2\# = 2, 3\# = 6, 5\# = 30, 7\# = 210, 11\# = 2310, 13\# = 30030$), and displays a compact primorial reference list.
  * **Record Gap Tracking**: Computes maximal record-breaking prime gaps $\Delta(n)$ and Cramér Ratios $C(n) = \frac{\Delta(n)}{(\ln p_n)^2}$.
  * **Residue Class Analysis**: Analyzes prime gap modulo alignments ($g \pmod 6$, $g \pmod{30}$).
  * **Markov Gap Transitions**: Analyzes 2-step gap transition probabilities $(g_n \to g_{n+1})$.

* **Comprehensive Test Suite**: Pure functional architecture with unit and CLI integration tests across core storage, sieve algorithms, analytics, and GUI state (`cargo test --workspace --all-targets`).

---

## 📁 Project Structure

```text
jumpchamp/
├── .devcontainer/
│   └── devcontainer.json        # VS Code Dev Container settings
├── crates/
│   ├── core/                    # jumpchamp-core: Headless engine & zero-copy storage
│   │   └── src/
│   │       ├── lib.rs           # Crate root — declares all core domain layers
│   │       ├── cancel.rs        # Cooperative atomic CancelToken for instant abort
│   │       ├── bin/
│   │       │   ├── generate_gaps.rs    # Offline prime sieve -> raw u16 gaps.bin CLI
│   │       │   └── generate_index.rs   # Offline gaps.bin -> block_index.bin CLI
│   │       ├── sieve/           # wheel.rs, parallel.rs, stream.rs (bitpacked wheel-of-30)
│   │       ├── analysis/        # gaps.rs, primorial.rs, report.rs (k-step combinators, factorizations, Cramér ratios)
│   │       ├── engine/          # histogram.rs, scan.rs, range.rs, frames.rs
│   │       └── storage/         # format.rs, raw.rs, engine.rs, builder.rs, spectral.rs
│   └── gui/                     # jumpchamp-gui: Native eframe/egui desktop application
│       └── src/
│           ├── main.rs          # Primary GUI entry point
│           ├── bin/jumpchamp_gui.rs # macOS bundle binary entry point
│           ├── actions.rs       # User action dispatcher and event loop
│           ├── app.rs           # eframe::App window shell, texture upload & update loop
│           ├── format.rs        # Formatting utilities (compact, thousands)
│           ├── playback.rs      # Pure animation state machine
│           ├── prefs.rs         # User preferences (theme, chart view, paths, limits)
│           ├── state.rs         # AppState, Query, Rank, DatasetMeta
│           ├── theme.rs         # Unified Palette definition & Viridis colormap
│           ├── worker.rs        # Background worker with CancelToken, epoch cancellation & mmap queries
│           ├── widgets/         # Reusable UI widgets (dual-thumb range slider, index input)
│           └── panels/          # Composable UI panels (chart, heatmap, playback_bar, settings, status_bar, toolbar)
├── scripts/
│   └── verify_all_features.sh   # Full fmt, clippy, test, and binary verification
└── Cargo.toml                   # Workspace root (crates/core, crates/gui)
```

Each domain layer is independently readable and testable:

| Layer | Modules | Responsibility | External Deps |
|---|---|---|---|
| `cancel` | `CancelToken` | Lock-free atomic cooperative cancellation token | none |
| `sieve/` | `wheel`, `parallel`, `stream` | Bitpacked wheel-of-30 prime generation & bounds | `rayon` |
| `analysis/` | `gaps`, `primorial`, `report` | Pure gap combinators, prime factorization, primorials ($p_n\#$), Cramér ratios & text reports | none |
| `engine/` | `histogram`, `scan`, `range`, `frames` | Gap histograms, chunk caching & animation frames | `sieve`, `analysis` |
| `storage/` | `format`, `raw`, `engine`, `builder`, `spectral` | Zero-copy mmap `gaps.bin` / `block_index.bin`, `DistributionEngine`, spectral matrix | `memmap2`, `bytemuck`, `rayon` |
| `gui/` | `state`, `worker`, `actions`, `panels`, `widgets` | Native desktop interface, `BarChart`, & 2D `ColorImage` spectral heatmap | `egui`, `eframe`, `egui_plot`, `crossbeam-channel` |

---

## 🚀 Quick Start

### Prerequisites
- **Rust** (1.70+ recommended) OR **VS Code with Dev Containers** extension.

### 1. Run from Source

Launch the native desktop GUI application with a single command:

```bash
cargo run --release
```

Or launch with pre-generated zero-copy binary files:

```bash
cargo run --release --bin jumpchamp -- --index block_index.bin --gaps gaps.bin
```

If `block_index.bin` (and optionally `gaps.bin`) exist in the working directory, JumpChamp auto-mounts them on startup in <1 ms; you can also mount or switch files anytime in **⚙ Settings -> Zero-Copy Binary Storage (Mmap)**.

### 2. Offline Zero-Copy Dataset & Block Index Generation

Build raw `u16` gaps (`gaps.bin`) and the two-tier `BlockHistogram` index (`block_index.bin`):

```bash
# 1. Sieve primes up to a prime bound and stream u16 1-step gaps to gaps.bin
cargo run --release -p jumpchamp-core --bin generate_gaps -- 100_000_000 gaps.bin

# 2. Scan gaps.bin in parallel and build block_index.bin (2.4 KB per 10M-prime block)
cargo run --release -p jumpchamp-core --bin generate_index -- gaps.bin block_index.bin
```

#### Engine Rust API (`DistributionEngine`)

```rust
use jumpchamp_core::storage::{DistributionEngine, RawGaps};

let engine = DistributionEngine::load("block_index.bin")?;
let dist: [u64; 600] = engine.query_distribution(0, 99_999_999);

let raw = RawGaps::open("gaps.bin")?;
let k3_hist = raw.k_step_histogram(0, 1_000_000, 3);
```

#### Data Footprint Summary ($10^{12}$ / ~37.6B Primes)

| Data Component | File Name | Size on Disk | Access Method |
| :--- | :--- | :--- | :--- |
| **Raw Gaps** | `gaps.bin` | ~75 GB (`u16`) | Zero-copy `memmap2` slice casting via `bytemuck` |
| **Block Index** | `block_index.bin` | ~9 MB ($3,761 \times 2.4\text{ KB}$) | Memory-mapped array of `BlockHistogram` structs |

### 3. macOS Installation via Homebrew

Install the standalone macOS desktop app:

```bash
brew install --cask JunghunLeePhD/tap/jumpchamp
```

> [!TIP]
> The Homebrew Cask includes an automated `postflight_steps` block that removes macOS quarantine flags upon installation, allowing JumpChamp to launch immediately without requiring manual security bypass commands.

---

## 🖥️ Key GUI Capabilities

- **Toolbar & View Modes**: Switch seamlessly between **📊 Bars** (`egui_plot::BarChart`), **🌈 Heatmap** (2D `egui::ColorImage` spectral density across prime-index scales), and **⊞ Split** view with synchronized gap focus lock.
- **Top-5 Ranking Board & Primorial HUD**: Top-left floating leaderboard (toggleable in Settings, with cursor hover tooltip box off by default) showing Rank #1–#5 gaps, prime factorizations, primorial tags ($p_n\#$), and a compact primorial reference table.
- **37.6B Prime Range Navigation**: Dual-thumb range sliders and compact index inputs supporting scales from `10M` up to `37.6B` ($\pi(10^{12})$) and `100B`.
- **Two-Tier Mmap + Sieve Engine**: Instant microsecond queries via `block_index.bin`, dynamic sliding-window $k$-step gaps via `gaps.bin`, and automatic fallback to the L1-cache parallel segmented sieve when no files are mounted.
- **Non-blocking Execution & Instant Reset**: Background worker channels, `CancelToken` polling across sieve/engine/mmap loops, and an always-enabled Reset button abort heavy runs mid-computation without UI freezes.

---

## 🛠️ Architecture & Multi-Crate Workspace

JumpChamp is organized as a Cargo multi-crate workspace separating the pure computational engine from the graphical user interface:

```
├── Cargo.toml                  # Virtual workspace root
├── crates/
│   ├── core/                   # jumpchamp-core: Headless prime gap engine (no GUI deps)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── sieve/          # Wheel-of-30, parallel segmented sieve, chunk streaming
│   │       ├── analysis/       # k-step gaps, transition matrices, residue counts, reports
│   │       └── engine/         # Histograms, chunk caching, range scans, animation frames
│   └── gui/                    # jumpchamp-gui: Native desktop application
│       ├── Cargo.toml
│       └── src/
│           ├── actions.rs      # User action reducer and dispatcher
│           ├── app.rs          # eframe application entry point and lifecycle
│           ├── panels/         # Toolbar, chart, playback bar, status bar, settings
│           ├── playback.rs     # Pure animation state machine (forward/reverse/bounce/step)
│           ├── state.rs        # AppState (Query, Rank, Prefs, Playback)
│           ├── widgets/        # Range slider, index input, tooltips
│           └── worker.rs       # Background worker channel with epoch cancellation
```

### Pure Functional Design Principles:
- **Zero-UI Core Engine**: `jumpchamp-core` has zero GUI dependencies (`rayon` only), compiles in <1 second, and can be embedded in CLIs, servers, or web services.
- **Pure Functions**: `small_primes` and `sieve_segment` are pure and free of side-effects, making them testable without external state.
- **Lazy Evaluation**: `stream_prime_blocks_range` streams blocks lazily, keeping memory bounded regardless of total primes generated.
- **Decoupled Worker Architecture**: The GUI main thread handles event loop rendering at 60 FPS while the computation worker executes in a separate thread communicating over crossbeam channels with epoch-synchronized `CancelToken` cooperative cancellation.

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
cargo bundle --release -p jumpchamp-gui --bin jumpchamp_gui

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
