// ============================================================================
// 2-Step Gap Database Storage — single-column gaps2.parquet (delta2: u16)
// ============================================================================

use arrow_array::{RecordBatch, UInt16Array};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::sync::Arc;

/// A write-only sink that stores single-column `delta2: u16` values in Parquet format.
///
/// **Single-column architecture**:
/// Row position 0..N-1 implicitly represents prime index n = 1..N.
/// Storing pre-computed Δ_2(n) = p_{n+2} - p_n directly allows DuckDB / analytical engines
/// to compute k=2 gap distributions with zero windowing and zero subtractions.
pub struct GapsSink2 {
    schema: Arc<Schema>,
    writer: ArrowWriter<File>,
}

impl GapsSink2 {
    /// Creates a new Parquet file at `path` ready for writing 2-step gap values.
    pub fn create(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("delta2", DataType::UInt16, false),
        ]));

        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(Default::default()))
            .build();

        let file = File::create(path)?;
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

        Ok(Self { schema, writer })
    }

    /// Writes a slice of `u16` 2-step gap values to the Parquet file.
    pub fn write_batch(&mut self, gaps: &[u16]) -> Result<(), Box<dyn std::error::Error>> {
        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![Arc::new(UInt16Array::from(gaps.to_vec()))],
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
