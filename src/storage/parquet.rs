// ============================================================================
// Parquet Metadata & I/O
// ============================================================================

use arrow_array::{RecordBatch, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use parquet::basic::{Compression, Encoding};
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Reads the maximum stored prime from Parquet row-group statistics in O(1) time.
///
/// Falls back to streaming all batches if statistics are unavailable.
pub fn get_existing_max_prime(path: &str) -> Option<u64> {
    if !Path::new(path).exists() {
        return None;
    }

    let file = File::open(path).ok()?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).ok()?;
    let metadata = builder.metadata();

    // 1. Try reading statistics from row group metadata (O(1))
    let mut max_val: Option<u64> = None;
    for rg in metadata.row_groups() {
        if let Some(stats) = rg.column(0).statistics() {
            if let parquet::file::statistics::Statistics::Int64(s) = stats {
                if let Some(&max) = s.max_opt() {
                    let max_u64 = max as u64;
                    max_val = Some(max_val.map_or(max_u64, |m| m.max(max_u64)));
                }
            }
        }
    }

    if max_val.is_some() {
        return max_val;
    }

    // 2. Fallback: stream batches to read last value
    let mut reader = builder.build().ok()?;
    let mut last_p = None;
    while let Some(Ok(batch)) = reader.next() {
        let col = batch.column(0).as_any().downcast_ref::<UInt64Array>()?;
        if !col.is_empty() {
            last_p = Some(col.value(col.len() - 1));
        }
    }
    last_p
}

// ----------------------------------------------------------------------------

/// A write-only sink that appends `u64` prime values to a Parquet file.
///
/// Uses Delta Binary Packed encoding + ZSTD compression (~1.3 bytes/prime).
pub struct ParquetPrimeSink {
    schema: Arc<Schema>,
    writer: ArrowWriter<File>,
}

impl ParquetPrimeSink {
    /// Creates a new Parquet file at `path` and opens it for writing.
    pub fn create(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Arc::new(Schema::new(vec![Field::new("prime", DataType::UInt64, false)]));

        let props = WriterProperties::builder()
            .set_column_encoding("prime".into(), Encoding::DELTA_BINARY_PACKED)
            .set_compression(Compression::ZSTD(Default::default()))
            .build();

        let file = File::create(path)?;
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

        Ok(Self { schema, writer })
    }

    /// Writes a pre-built `RecordBatch` directly (used when copying existing data).
    pub fn write_record_batch(&mut self, batch: &RecordBatch) -> Result<(), Box<dyn std::error::Error>> {
        self.writer.write(batch)?;
        Ok(())
    }

    /// Converts a slice of `u64` primes into a `RecordBatch` and writes it.
    pub fn write_batch(&mut self, primes: &[u64]) -> Result<(), Box<dyn std::error::Error>> {
        let array = Arc::new(UInt64Array::from(primes.to_vec()));
        let batch = RecordBatch::try_new(self.schema.clone(), vec![array])?;
        self.writer.write(&batch)?;
        Ok(())
    }

    /// Flushes all buffered data and closes the file. Must be called to finalize the file.
    pub fn finish(self) -> Result<(), Box<dyn std::error::Error>> {
        self.writer.close()?;
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// Copies all existing batches from an existing Parquet file into `sink`.
///
/// Returns the total number of primes copied.
pub fn copy_existing_parquet(
    input_path: &str,
    sink: &mut ParquetPrimeSink,
) -> Result<usize, Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;
    let mut count = 0;

    for batch in reader {
        let batch = batch?;
        let col = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .ok_or("Invalid schema in existing parquet file")?;

        count += col.len();
        sink.write_record_batch(&batch)?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parquet_sink_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let tmp_path = ".test_primes_roundtrip.parquet";

        let primes = vec![2u64, 3, 5, 7, 11, 13, 17, 19];
        {
            let mut sink = ParquetPrimeSink::create(tmp_path)?;
            sink.write_batch(&primes)?;
            sink.finish()?;
        }

        let max_prime = get_existing_max_prime(tmp_path);
        assert_eq!(max_prime, Some(19));

        if Path::new(tmp_path).exists() {
            std::fs::remove_file(tmp_path)?;
        }
        Ok(())
    }
}

