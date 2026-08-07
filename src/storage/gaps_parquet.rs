// ============================================================================
// Gap Database Storage — writes gaps.parquet (prime, Δ_1(n)) pairs
// ============================================================================

use arrow_array::{RecordBatch, UInt16Array, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, Encoding};
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::sync::Arc;

/// A write-only sink that stores `(prime: u64, gap: u16)` pairs in Parquet format.
///
/// The `prime` column holds p_n and `gap` holds Δ_1(n) = p_{n+1} − p_n.
/// Storing 1-step gaps is the universal primitive: k-step gaps for any k
/// are derived as a sliding sum of k consecutive 1-step gap values.
///
/// **Encoding**: Delta Binary Packed on both columns + ZSTD.
/// Expected ~3–4 MB for 100M primes vs ~7.2 MB for primes.parquet.
pub struct GapsSink {
    schema: Arc<Schema>,
    writer: ArrowWriter<File>,
}

impl GapsSink {
    /// Creates a new Parquet file at `path` ready for writing gap pairs.
    pub fn create(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("prime", DataType::UInt64, false),
            Field::new("gap",   DataType::UInt16, false),
        ]));

        let props = WriterProperties::builder()
            .set_column_encoding("prime".into(), Encoding::DELTA_BINARY_PACKED)
            .set_column_encoding("gap".into(),   Encoding::DELTA_BINARY_PACKED)
            .set_compression(Compression::ZSTD(Default::default()))
            .build();

        let file = File::create(path)?;
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

        Ok(Self { schema, writer })
    }

    /// Writes a batch of `(prime, gap)` pairs to the Parquet file.
    pub fn write_batch(&mut self, pairs: &[(u64, u16)]) -> Result<(), Box<dyn std::error::Error>> {
        let primes: Vec<u64> = pairs.iter().map(|&(p, _)| p).collect();
        let gaps:   Vec<u16> = pairs.iter().map(|&(_, g)| g).collect();

        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(UInt64Array::from(primes)),
                Arc::new(UInt16Array::from(gaps)),
            ],
        )?;
        self.writer.write(&batch)?;
        Ok(())
    }

    /// Flushes all buffered data, writes the Parquet footer, and closes the file.
    pub fn finish(self) -> Result<(), Box<dyn std::error::Error>> {
        self.writer.close()?;
        Ok(())
    }
}
