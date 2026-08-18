# Jump Champ 🦀📊

A high-performance, multi-threaded Rust pipeline designed to generate billions of prime numbers, store them in highly compressed **Parquet** files using **Delta Binary Packed Encoding**, and perform zero-copy stream analysis on prime gap distributions ($p_{n+k} - p_n$).

Built with a functional programming architecture, cache-aligned multi-threading via **Rayon**, and **VS Code Dev Containers** support.

---

## 🌟 Key Features

* **Native Desktop GUI (`egui` + `eframe`)**: Cross-platform desktop interface featuring non-blocking Parquet I/O, GPU-accelerated plots (`egui_plot`), virtualized table inspection (`egui_extras`), and real-time LTTB data downsampling for smooth 60+ FPS rendering of datasets with millions of rows.

* **Parallel Bitpacked Segmented Sieve**: Utilizes Rayon to process 64-bit bitmasked odd-only segments in parallel, fitting 262,144 candidates into CPU L1-cache for an **8x-16x memory footprint reduction**.

* **Delta-Packed Parquet Sink**: Stores primes using Delta Binary Packed encoding + ZSTD compression, reducing storage from **8 bytes/prime** down to **~1.3 bytes/prime** (~84% space reduction).
* **Lazy $O(1)$ Memory Streaming**: Streams blocks directly to disk without keeping billions of integers in RAM.

* **Advanced Mathematical Analytics**:
  * **Record Gap Tracking**: Computes maximal record-breaking prime gaps $\Delta(n)$ and Cramér Ratios $C(n) = \frac{\Delta(n)}{(\ln p_n)^2}$.
  * **Residue Class Analysis**: Analyzes prime gap modulo alignments ($g \pmod 6$, $g \pmod{30}$).
  * **Markov Gap Transitions**: Analyzes 2-step gap transition probabilities $(g_n \to g_{n+1})$.

* **Comprehensive Test Suite**: Pure functional architecture with zero-filesystem unit tests for algorithms, iterator combinators, LTTB downsampling, and Parquet round-trips (`cargo test`).

* **SQL & DuckDB Ready**: Interoperable with DuckDB, Python (Pandas/Polars), and standard Apache Arrow tooling.

## 📁 Project Structure

```text
jumpchamp/
├── .devcontainer/
│   └── devcontainer.json        # VS Code Dev Container settings
├── src/
│   ├── lib.rs                   # Library root — declares all domain layers
│   ├── config.rs                # Config (generator) & AnalyzeConfig (analyzer)
│   ├── main.rs                  # Analyzer binary entry point (thin shell)
│   ├── sieve/
│   │   ├── mod.rs               # Re-exports basic, parallel, stream
│   │   ├── basic.rs             # small_primes, sieve_segment (bitpacked odd-only sieve)
│   │   ├── parallel.rs          # sieve_range_parallel (Rayon L1-cache bitmask dispatcher)
│   │   └── stream.rs            # stream_prime_blocks_range (lazy block iterator)
│   ├── storage/
│   │   ├── mod.rs               # Re-exports parquet, gaps_parquet
│   │   ├── parquet.rs           # ParquetPrimeSink, get_existing_max_prime, copy_existing_parquet
│   │   └── gaps_parquet.rs      # GapsSink for storing single-column (deltak: u16) pairs
│   ├── analysis/
│   │   ├── mod.rs               # Re-exports gaps, report
│   │   ├── gaps.rs              # stream_primes, apply_interval, k_step_gaps, record_gaps, count_residues, gap_transition_matrix
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
│       ├── build_primes.rs      # Prime database builder binary
│       ├── build_gaps.rs        # Gap database builder binary
│       └── jumpchamp_gui.rs     # Native desktop GUI entry point binary
├── jumpchamp_web/               # Streamlit web application package
│   ├── __init__.py              # Web package root with clean exports
│   ├── config.py                # Domain configuration types & loaders
│   ├── ingestion.py             # Resumable Range downloads & URL resolution
│   ├── database.py              # DuckDB query engine & data processing
│   ├── components.py            # Streamlit UI components & Plotly charts
│   └── runner.py                # Main application orchestrator
├── app.py                       # Streamlit dashboard entry point (defaults to k=2)
├── app2.py                      # 2-Step Prime Gap Explorer (k=2)
├── app3.py                      # 3-Step Prime Gap Explorer (k=3)
├── app_common.py                # Backward-compatible re-export facade
├── .gitignore                   # Ignores /target and *.parquet artifacts
└── Cargo.toml                   # Dependencies (Rayon, Arrow, Parquet, egui, eframe)
```

Each domain layer is independently readable and testable:

| Layer | Modules | Responsibility | External Deps |
|-------|---------|---------------|---------------|
| `sieve/` | `basic`, `parallel`, `stream` | Bitpacked odd-only prime generation | `rayon` |
| `storage/` | `parquet`, `gaps_parquet` | Parquet read/write sinks | `arrow`, `parquet` crates |
| `analysis/` | `gaps`, `report` | Gap analysis, Cramér ratios, residues & report formatting | `parquet` crate (reader) |
| `gui/` | `app`, `state`, `worker`, `lttb`, `theme`, `panels` | Native desktop interface & virtualized rendering | `egui`, `eframe`, `egui_plot`, `egui_extras`, `crossbeam-channel`, `rfd` |
| _(top-level)_ | `config` | CLI arg parsing for the generator | none |

## **🚀 Quick Start**

### **Prerequisites**

- **Rust** (1.70+ recommended) OR **VS Code with Dev Containers** extension.

### **1. Open in Dev Container (Recommended)**

1. Open the project in VS Code.
2. Press `F1` and select **Dev Containers: Reopen in Container**.


## **💻 Usage**

### **1. Building the Prime Database (`build_primes`)**

Generates all primes up to a target upper limit N and writes them to `primes.parquet`.

```bash
# Default limit: 10,000,000
cargo run --release --bin build_primes
```

```bash
# Custom limit: Primes up to 100,000,000
cargo run --release --bin build_primes -- 100000000
```

#### **Output Metrics Example**

```plaintext
Creating new prime database up to 100000000...

----------------------------------------
Total Primes in DB: 5761455
Time Elapsed:       1.42s
Parquet File Size:  7.21 MB
Compression Ratio:  1.31 bytes/prime
----------------------------------------
```

### **2. Building Pre-Computed Gap Databases (`build_gaps`)**

Pre-computes $k$-step prime gaps ($\Delta_k(n) = p_{n+k} - p_n$) as a **single-column `deltak: u16` Parquet file** (`gaps{k}.parquet`, ~2048 MB).

```bash
# Default (k=2): Builds gaps2.parquet
cargo run --release --bin build_gaps

# Custom step size k=3: Builds gaps3.parquet
cargo run --release --bin build_gaps -- 3

# Custom step size k=6 with custom input/output paths:
cargo run --release --bin build_gaps -- 6 primes.parquet custom.parquet
```

### **3. Gap Distribution Analyzer (`main.rs` / default binary)**

Analyzes prime gap distributions ($p_{n+k} - p_n$) over a specified prime index interval $[n, m]$.

> [!IMPORTANT]
> **Mandatory Gaps Database Requirement**: By default, the CLI analyzer requires the pre-computed gaps database (`gaps{k}.parquet` or `gaps.parquet`). If missing, run `cargo run --release --bin build_gaps -- {k}` first, or pass `--force` to calculate directly from `primes.parquet` (slow path).

```bash
# Default run: k=2 (2-step gaps) using pre-computed gaps2.parquet
cargo run --release
```

#### **CLI Syntax & Arguments**

```bash
cargo run --release -- [k] [min_idx] [max_idx] [primes_file] [--force]
```

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `k` | `usize` | `2` | Step size $k$ for gap calculation ($p_{n+k} - p_n$) |
| `min_idx` | `u64` | `1` | Start prime index $n$ (1-based inclusive) |
| `max_idx` | `u64` | `u64::MAX` | End prime index $m$ (1-based inclusive) |
| `primes_file` | `String` | `primes.parquet` | Input prime database path (or derived `gaps{k}.parquet`) |
| `--force` / `-f` | Flag | `false` | Force execution using `primes.parquet` (slow path) if gaps file is missing |

#### **Execution Examples**

```bash
# Analyze 2-step gaps using pre-computed gaps2.parquet
cargo run --release -- 2 1 1000000

# Analyze 3-step gaps (k=3) using pre-computed gaps3.parquet
cargo run --release -- 3 1 1000000

# Force direct calculation from primes.parquet without pre-built gaps file
cargo run --release -- 2 1 1000000 --force
```

#### **Execution Engine**

- ⚡ **Fast Path (`gaps{k}.parquet` / `gaps.parquet` present)**: Default mode. Streams single-column 16-bit integers (`u16`) via `stream_gaps` with offset slicing for zero-copy high-speed analysis (~95 MB RAM).
- 🐢 **Slow Path (`--force` flag)**: Triggered when `--force` (or `-f`) is supplied. Streams 64-bit primes on the fly from `primes.parquet` and evaluates $k$-step gaps via sliding window combinators.

### **4. Native Desktop GUI (`jumpchamp_gui`)**

Launch the native `egui` desktop GUI application for GPU-accelerated interactive histogram & scatter charts, zero-lag virtualized data tables, and live file loading:

```bash
cargo run --release --bin jumpchamp_gui
```

Key GUI Capabilities:
- **3-Panel Workflow**: Sidebar controls, interactive `egui_plot` visualizer, and `egui_extras::TableBuilder` virtualized row inspection.
- **LTTB Downsampling**: Dynamically reduces $10^7$+ rows down to ~2,000 display points for real-time panning/zooming at 60+ FPS.
- **Non-blocking Execution**: Background worker channel prevents UI freezes while streaming `.parquet` files.

### **5. Web UI Dashboards (`app2.py` & `app3.py`)**

Dedicated Streamlit dashboards specialized for real-time visualization of 2-step ($\Delta_2$) and 3-step ($\Delta_3$) prime gap distributions. Each application loads its respective single-column Parquet database (`gaps2.parquet` or `gaps3.parquet`) with zero windowing operator overhead and zero subtractions.

```bash
# Run 2-Step Prime Gap Explorer (k=2)
streamlit run app2.py

# Run 3-Step Prime Gap Explorer (k=3)
streamlit run app3.py
```

### **6. Querying with DuckDB**

Because the output is standard Parquet, you can run SQL queries directly on `primes.parquet`:

```bash
# Total count and largest prime
duckdb -c "SELECT COUNT(*) AS count, MAX(prime) AS max_prime FROM 'primes.parquet';"

# Find twin primes (p, p+2)
duckdb -c "
  SELECT p1.prime AS p1, p2.prime AS p2 
  FROM 'primes.parquet' p1 
  JOIN 'primes.parquet' p2 ON p2.prime = p1.prime + 2 
  LIMIT 10;
"
```

## **🤖 Automated GitHub Actions Database Releases**

This repository includes a GitHub Actions workflow ([`.github/workflows/build-release-db.yml`](file:///workspace/.github/workflows/build-release-db.yml)) that automatically builds the prime & gap Parquet files and attaches them directly to GitHub Releases.

### **How to Trigger Automated Builds**

1. **Manual Trigger (GitHub UI):**
   * Go to the **Actions** tab in your GitHub repository.
   * Select **Auto-Build & Publish Parquet Database Release**.
   * Click **Run workflow** and specify target `prime_limit` (e.g. `100000000`), `gap_k` (e.g. `2`), and `release_tag` (e.g. `v1.0.0`).

2. **Automatic Trigger on Tag Push:**
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```
   GitHub Actions will automatically run unit tests, build `primes.parquet` and `gaps2.parquet`, and upload them directly to the `v1.0.0` release assets.

## **🛠️ Architecture & Functional Design**

The project strictly follows Functional Programming (FP) principles:

- **Pure Functions**: `small_primes` and `sieve_segment` are pure and free of side-effects, making them easily unit-testable without filesystem access.

- **Lazy Evaluation**: `stream_prime_blocks` streams 107 element chunks lazily, keeping memory bounded regardless of total primes generated.

- **Stream Pipeline Composition**: The gap analyzer composes modular iterator functions:

    $\text{Frequencies} = \text{count\_frequencies} \circ \text{k\_step\_gaps} \circ \text{apply\_interval} \circ \text{stream\_primes}$

## **📦 Distributing Standalone Executables to Friends**

The app is built as a single, standalone native executable with zero external runtime dependencies. Friends do **not** need Rust installed.

### **Building the GUI App Locally (Your Own Machine)**

To build for your current machine, simply run — no `--target` flag needed:

```bash
cargo build --release --bin jumpchamp_gui
```

The output binary will appear in `target/release/`:
- `target/release/jumpchamp_gui.exe` (Windows)
- `target/release/jumpchamp_gui` (macOS / Linux)

> [!IMPORTANT]
> **Cross-compiling for other platforms** (e.g. building a macOS binary from Linux inside the Dev Container) requires target-specific SDK toolchains and cross-linkers.
> **Use GitHub Actions CI instead** — it builds natively on each platform automatically. See below.

---

### **Building Platform-Specific Binaries (On That Native Platform)**

These commands should be run **on the target platform itself**:

| Platform | Prerequisite | Build Command |
| :--- | :--- | :--- |
| **Windows** | Windows OS + Rust installed | `cargo build --release --bin jumpchamp_gui` |
| **macOS (Apple Silicon)** | macOS arm64 + `rustup target add aarch64-apple-darwin` | `cargo build --release --target aarch64-apple-darwin --bin jumpchamp_gui` |
| **macOS (Intel)** | macOS x86_64 + `rustup target add x86_64-apple-darwin` | `cargo build --release --target x86_64-apple-darwin --bin jumpchamp_gui` |
| **Linux** | Linux x86_64 + Rust installed | `cargo build --release --bin jumpchamp_gui` |

---

### **🖼️ Setting Application & Desktop Icons**

The app displays its icon in the runtime OS Dock/Taskbar, as well as on Desktop and File Managers:

* **Windows (`.exe` File & Desktop Icon)**:
  `build.rs` embeds `assets/icon.ico` directly into `jumpchamp_gui.exe` using `winres`. When compiling on Windows (`cargo build --release`), the resulting executable displays the JumpChamp icon on the Desktop and in File Explorer.
* **Linux (Desktop Launcher & Menu Icon)**:
  Linux desktop environments (GNOME, KDE, XFCE) read `.desktop` launcher files. Run the included helper script to install the desktop shortcut and high-res icon:
  ```bash
  ./install_desktop_shortcut.sh
  ```
  This installs `JumpChamp` to your Application Menu and places a launchable shortcut on `~/Desktop`.
* **macOS (`.app` Bundle Icon)**:
  macOS uses `.app` bundles configured via `[package.metadata.bundle]` in `Cargo.toml`. Building with `cargo-bundle` automatically packages `JumpChamp.app` with `AppIcon.icns`:
  ```bash
  cargo install cargo-bundle
  cargo bundle --release --bin jumpchamp_gui
  ```



---

## **🤖 Automated GitHub Actions GUI Releases**

This repository features an automated GitHub Actions CI/CD pipeline ([`.github/workflows/release-gui.yml`](file:///workspace/.github/workflows/release-gui.yml)).

Whenever a version tag is pushed (e.g. `v1.0.0`), GitHub Actions automatically builds standalone executables for **Windows**, **macOS**, and **Linux** and attaches them directly to the GitHub Release page:

```bash
git tag v1.0.0
git push origin v1.0.0
```

### **How Friends Can Download and Run:**
1. Go to your GitHub Repository **Releases** page.
2. Download the binary for their OS (`jumpchamp_gui-windows-x86_64.exe`, `jumpchamp_gui-macos-arm64`, or `jumpchamp_gui-linux-x86_64`).
3. Double-click to play!

---

## **📊 Performance Benchmarks**

| **Metric** | **Raw UInt64 Array** | **Parquet (Delta + ZSTD)** |
| :--- | :--- | :--- |
| **Storage per Prime** | 8.0 bytes | **~1.3 bytes** |
| **100M Limit File Size** | 46.09 MB | **7.21 MB** |
| **Analysis Throughput** | — | **>70M primes/sec** |

## **📜 License**

MIT License. Feel free to use and modify for analytical and educational research.


