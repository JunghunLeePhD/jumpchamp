# Jump Champ 🦀📊

A high-performance, multi-threaded Rust pipeline designed to generate billions of prime numbers, store them in highly compressed **Parquet** files using **Delta Binary Packed Encoding**, and perform zero-copy stream analysis on prime gap distributions ($p_{n+k} - p_n$).

Built with a functional programming architecture, cache-aligned multi-threading via **Rayon**, and **VS Code Dev Containers** support.

---

## 🌟 Key Features

* **Parallel Segmented Sieve**: Utilizes Rayon to process 32 KB CPU L1-cache-aligned blocks in parallel.

* **Delta-Packed Parquet Sink**: Stores primes using Delta Binary Packed encoding + ZSTD compression, reducing storage from **8 bytes/prime** down to **~1.3 bytes/prime** (~84% space reduction).
* **Lazy $O(1)$ Memory Streaming**: Streams blocks directly to disk without keeping billions of integers in RAM.

* **FP Architecture**: Clean functional separation of pure core algorithms, lazy stream generators, and I/O sinks.

* **Interval & Gap Analyzer (`analyze`)**: Secondary binary for computing $k$-step prime gap distributions ($p_{n+k} - p_n$) over arbitrary intervals $[A, B]$ with early exit optimizations.

* **SQL & DuckDB Ready**: Interoperable with DuckDB, Python (Pandas/Polars), and standard Apache Arrow tooling

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
│   │   ├── basic.rs             # small_primes, sieve_segment (pure, sequential)
│   │   ├── parallel.rs          # sieve_range_parallel (Rayon, L1-cache-aligned)
│   │   └── stream.rs            # stream_prime_blocks_range (lazy block iterator)
│   ├── storage/
│   │   ├── mod.rs               # Re-exports parquet, gaps_parquet
│   │   ├── parquet.rs           # ParquetPrimeSink, get_existing_max_prime, copy_existing_parquet
│   │   └── gaps_parquet.rs      # GapsSink for storing (prime, gap) pairs
│   ├── analysis/
│   │   ├── mod.rs               # Re-exports gaps, report
│   │   ├── gaps.rs              # stream_primes, apply_interval, k_step_gaps, count_frequencies, stream_gap_pairs, apply_gap_interval, k_step_gaps_from_pairs
│   │   └── report.rs            # format_report (pure text formatter)
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
| `sieve/` | `basic`, `parallel`, `stream` | Prime generation algorithms | `rayon` |
| `storage/` | `parquet`, `gaps_parquet` | Parquet read/write sinks | `arrow`, `parquet` crates |
| `analysis/` | `gaps`, `report` | Gap analysis pipeline & formatting | `parquet` crate (reader) |
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

### **3. Web UI Dashboard (`app.py`)**

Streamlit dashboard specialized for real-time visualization of 2-step prime gap distributions ($\Delta_2(n) = p_{n+2} - p_n$, $k=2$). Queries the single-column `gaps2.parquet` database (~90 MB) with zero windowing operator overhead and zero subtractions.

```bash
streamlit run app.py
```

### **4. Querying with DuckDB**

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
