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

* **SQL & DuckDB Ready**: Interoperable with DuckDB, Python (Pandas/Polars), and standard Apache Arrow tooling.

---

## 📁 Project Structure

```text
prime_pipeline/
├── .devcontainer/
│   └── devcontainer.json    # VS Code Dev Container settings (Clippy, LLDB, Rust-Analyzer)
├── src/
│   ├── main.rs              # Prime generator & Parquet writer
│   └── bin/
│       └── analyze.rs       # Stream analyzer for prime gap distributions (p_{n+k} - p_n)
├── .gitignore               # Ignores /target and *.parquet artifacts
└── Cargo.toml               # Dependencies (Rayon, Arrow, Parquet)
```

## **🚀 Quick Start**

### **Prerequisites**

- **Rust** (1.70+ recommended) OR **VS Code with Dev Containers** extension.

### **1. Open in Dev Container (Recommended)**

1. Open the project in VS Code.

2. Press `F1` and select **Dev Containers: Reopen in Container**.


## **💻 Usage**

### **1. Generating Primes (`**main.rs**`)**

Generates all primes up to a target upper limit N and writes them to `primes.parquet`.

```bash
# Default limit: 10,000,000
cargo run --release
```

```
# Custom limit: Primes up to 100,000,000
cargo run --release -- 100000000
```

#### **Output Metrics Example**

```plaintext
Generating primes up to 100000000 -> primes.parquet

----------------------------------------
Total Primes:      5,761,455
Time Elapsed:      1.42s
Parquet File Size: 7.21 MB
Compression Ratio: 1.31 bytes/prime
----------------------------------------
```

### **2. Analyzing Prime Gap Distributions (`**analyze.rs**`)**

Analyze the frequency distribution of k-step prime differences (pn+k​−pn​) across an optional numerical interval [A,B].

#### **Command Syntax**

```bash
cargo run --release --bin analyze -- [k] [min_prime] [max_prime] [parquet_file]
```

#### **Examples**

```bash
# 1-step gap (p_{n+1} - p_n) across all generated primes
cargo run --release --bin analyze -- 1

# 2-step gap (p_{n+2} - p_n) for primes in interval [1,000,000, 10,000,000]
cargo run --release --bin analyze -- 2 1000000 10000000

# 6-step gap (p_{n+6} - p_n) on a custom parquet file
cargo run --release --bin analyze -- 6 0 100000000 path/to/primes.parquet
```

#### **Analysis Output Example**

```plaintext
Analyzing prime gaps (p_{n+2} - p_n)
Interval:  [1000000, 10000000]
File:      primes.parquet

Diff         Frequency       Percentage  
------------------------------------------
6            124,581         21.62%
12           89,240          15.49%
8            62,310          10.81%
10           59,102          10.26%
------------------------------------------
Processed 576,145 prime pairs in 82.4ms
```

### **3. Querying with DuckDB**

Because the output is standard Parquet, you can run SQL queries directly on `primes.parquet`:

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
