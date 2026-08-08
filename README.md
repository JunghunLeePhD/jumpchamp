# Jump Champ 🦀📊

A high-performance, multi-threaded Rust pipeline designed to generate billions of prime numbers, store them in highly compressed **Parquet** files using **Delta Binary Packed Encoding**, and perform zero-copy stream analysis on prime gap distributions ($p_{n+k} - p_n$).

Built with a functional programming architecture, cache-aligned multi-threading via **Rayon**, and **VS Code Dev Containers** support.

---

## 🌟 Key Features

* **Parallel Bitpacked Segmented Sieve**: Utilizes Rayon to process 64-bit bitmasked odd-only segments in parallel, fitting 262,144 candidates into CPU L1-cache for an **8x-16x memory footprint reduction**.

* **Delta-Packed Parquet Sink**: Stores primes using Delta Binary Packed encoding + ZSTD compression, reducing storage from **8 bytes/prime** down to **~1.3 bytes/prime** (~84% space reduction).
* **Lazy $O(1)$ Memory Streaming**: Streams blocks directly to disk without keeping billions of integers in RAM.

* **Advanced Mathematical Analytics**:
  * **Record Gap Tracking**: Computes maximal record-breaking prime gaps $\Delta(n)$ and Cramér Ratios $C(n) = \frac{\Delta(n)}{(\ln p_n)^2}$.
  * **Residue Class Analysis**: Analyzes prime gap modulo alignments ($g \pmod 6$, $g \pmod{30}$).
  * **Markov Gap Transitions**: Analyzes 2-step gap transition probabilities $(g_n \to g_{n+1})$.

* **Comprehensive Test Suite**: Pure functional architecture with zero-filesystem unit tests for algorithms, iterator combinators, and Parquet round-trips (`cargo test`).

* **SQL & DuckDB Ready**: Interoperable with DuckDB, Python (Pandas/Polars), and standard Apache Arrow tooling.

## 📁 Project Structure

```text
jumpchamp/
├── .devcontainer/
│   └── devcontainer.json        # VS Code Dev Container settings
├── src/
│   ├── lib.rs                   # Library root — declares all domain layers
│   ├── config.rs                # Config struct & CLI arg parsing (generator)
│   ├── main.rs                  # Analyzer binary (default entry point) — thin shell
│   ├── sieve/
│   │   ├── mod.rs               # Re-exports basic, parallel, stream
│   │   ├── basic.rs             # small_primes, sieve_segment (bitpacked odd-only sieve)
│   │   ├── parallel.rs          # sieve_range_parallel (Rayon L1-cache bitmask dispatcher)
│   │   └── stream.rs            # stream_prime_blocks_range (lazy block iterator)
│   ├── storage/
│   │   ├── mod.rs               # Re-exports parquet, gaps_parquet
│   │   ├── parquet.rs           # ParquetPrimeSink, get_existing_max_prime, copy_existing_parquet
│   │   └── gaps_parquet.rs      # GapsSink for storing (prime, gap) pairs
│   ├── analysis/
│   │   ├── mod.rs               # Re-exports gaps, report
│   │   ├── gaps.rs              # stream_primes, apply_interval, k_step_gaps, record_gaps, count_residues, gap_transition_matrix
│   │   └── report.rs            # format_report, format_record_gaps_report, format_residue_report
│   └── bin/
│       ├── build_primes.rs      # Prime database builder binary
│       └── build_gaps.rs        # Gap database builder binary
├── app.py                       # Streamlit dashboard (DuckDB-backed, prime gap visualization)
├── .gitignore                   # Ignores /target and *.parquet artifacts
└── Cargo.toml                   # Dependencies (Rayon, Arrow, Parquet)
```

Each domain layer is independently readable and testable:

| Layer | Modules | Responsibility | External Deps |
|-------|---------|---------------|---------------|
| `sieve/` | `basic`, `parallel`, `stream` | Bitpacked odd-only prime generation | `rayon` |
| `storage/` | `parquet`, `gaps_parquet` | Parquet read/write sinks | `arrow`, `parquet` crates |
| `analysis/` | `gaps`, `report` | Gap analysis, Cramér ratios, residues & report formatting | `parquet` crate (reader) |
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

Pre-computes $k$-step prime gaps ($\Delta_k(n) = p_{n+k} - p_n$) as a **single-column `deltak: u16` Parquet file** (`gaps{k}.parquet`, ~90 MB).

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

```bash
# Default run: k=2 (2-step gaps), all prime indices [1, ∞], using primes.parquet / gaps.parquet
cargo run --release
```

#### **CLI Syntax & Positional Arguments**

```bash
cargo run --release -- [k] [min_idx] [max_idx] [primes_file]
```

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `k` | `usize` | `2` | Step size $k$ for gap calculation ($p_{n+k} - p_n$) |
| `min_idx` | `u64` | `1` | Start prime index $n$ (1-based inclusive) |
| `max_idx` | `u64` | `u64::MAX` | End prime index $m$ (1-based inclusive) |
| `primes_file` | `String` | `primes.parquet` | Input prime database path (or derived `gaps.parquet`) |

#### **Execution Examples**

```bash
# Analyze 2-step gaps for prime indices 1 to 1,000,000
cargo run --release -- 2 1 1000000

# Analyze 4-step gaps (k=4) for primes between index 100,000 and 500,000
cargo run --release -- 4 100000 500000

# Specify a custom primes parquet database path
cargo run --release -- 2 1 1000000 /path/to/custom_primes.parquet
```

#### **Dual Execution Engine**

- ⚡ **Fast Path (`gaps.parquet` present)**: Automatically detects `gaps.parquet` and streams single-column 16-bit integers (`u16`) via `stream_gaps` with offset slicing for zero-copy high-speed analysis (~95 MB RAM).
- 🐢 **Slow Path (`primes.parquet` fallback)**: If `gaps.parquet` is missing, streams 64-bit primes on the fly from `primes.parquet` and evaluates $k$-step gaps via sliding window combinators.

### **4. Web UI Dashboard (`app.py`)**

Streamlit dashboard specialized for real-time visualization of 2-step prime gap distributions ($\Delta_2(n) = p_{n+2} - p_n$, $k=2$). Queries the single-column `gaps2.parquet` database (~90 MB) with zero windowing operator overhead and zero subtractions.

```bash
streamlit run app.py
```

### **5. Querying with DuckDB**

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

## **📊 Performance Benchmarks**

| **Metric** | **Raw UInt64 Array** | **Parquet (Delta + ZSTD)** |
| :--- | :--- | :--- |
| **Storage per Prime** | 8.0 bytes | **~1.3 bytes** |
| **100M Limit File Size** | 46.09 MB | **7.21 MB** |
| **Analysis Throughput** | — | **>70M primes/sec** |

## **📜 License**

MIT License. Feel free to use and modify for analytical and educational research.

