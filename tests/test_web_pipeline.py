#!/usr/bin/env python3
"""
JumpChamp Web Pipeline & DuckDB Integration Test Suite.
Verifies all Python components, DuckDB queries, configuration loaders,
and data transformations.
"""

import os
import unittest
import duckdb
import pandas as pd

from jumpchamp_web.config import load_config, AppConfig, FilterParams, DatasetMetadata
from jumpchamp_web.database import (
    get_db_connection,
    fetch_dataset_metadata,
    query_prime_gaps,
    process_gap_dataframe,
)


class TestWebConfiguration(unittest.TestCase):
    """Verifies domain configuration types and loaders."""

    def test_load_config_k2(self):
        config = load_config(2)
        self.assertIsInstance(config, AppConfig)
        self.assertEqual(config.gaps_file, "gaps2.parquet")
        self.assertIn("gaps2.parquet", config.release_url)

    def test_load_config_k3(self):
        config = load_config(3)
        self.assertIsInstance(config, AppConfig)
        self.assertEqual(config.gaps_file, "gaps3.parquet")
        self.assertIn("gaps3.parquet", config.release_url)

    def test_filter_params_immutability(self):
        params = FilterParams(min_idx=1, max_idx=1000, top_min=1, top_max=10, sort_by="Frequency")
        self.assertEqual(params.min_idx, 1)
        self.assertEqual(params.max_idx, 1000)
        with self.assertRaises(Exception):
            params.min_idx = 10  # Frozen dataclass


class TestDuckDBEngineWithMockParquet(unittest.TestCase):
    """Verifies DuckDB query engine and data processing on a controlled Parquet dataset."""

    @classmethod
    def setUpClass(cls):
        cls.test_parquet = "test_gaps_fixture.parquet"
        # Create a small sample parquet file with deltak column
        sample_gaps = [2, 4, 2, 4, 6, 2, 6, 4, 2, 4, 6, 6, 2, 6, 4, 2, 6, 4, 6, 8, 4, 2, 4, 2, 4, 14, 4]
        df = pd.DataFrame({"deltak": sample_gaps}, dtype="uint16")
        df.to_parquet(cls.test_parquet, engine="pyarrow", compression="zstd")

        cls.conn = duckdb.connect()
        cls.conn.sql("SET max_memory = '512MB';")
        cls.conn.sql("SET threads = 2;")

    @classmethod
    def tearDownClass(cls):
        if os.path.exists(cls.test_parquet):
            try:
                os.remove(cls.test_parquet)
            except OSError:
                pass

    def test_fetch_metadata(self):
        meta = fetch_dataset_metadata(self.conn, self.test_parquet)
        self.assertIsInstance(meta, DatasetMetadata)
        self.assertEqual(meta.min_idx, 1)
        self.assertEqual(meta.max_idx, 27)
        self.assertEqual(meta.total_count, 27)
        self.assertEqual(meta.unique_gaps_count, 5)  # 2, 4, 6, 8, 14

    def test_query_prime_gaps_sorted_by_frequency(self):
        params = FilterParams(min_idx=1, max_idx=27, top_min=1, top_max=5, sort_by="Frequency")
        df = query_prime_gaps(self.conn, self.test_parquet, params)
        self.assertFalse(df.empty)
        self.assertIn("diff", df.columns)
        self.assertIn("frequency", df.columns)

        processed = process_gap_dataframe(df, sort_by="Frequency")
        self.assertIn("percentage", processed.columns)
        self.assertIn("diff_label", processed.columns)

        # Check percentages sum to approximately 100%
        self.assertAlmostEqual(processed["percentage"].sum(), 100.0, delta=1.0)
        # Check descending order by frequency
        frequencies = processed["frequency"].tolist()
        self.assertEqual(frequencies, sorted(frequencies, reverse=True))

    def test_query_prime_gaps_sorted_by_gap_size(self):
        params = FilterParams(min_idx=1, max_idx=27, top_min=1, top_max=5, sort_by="Gap Size")
        df = query_prime_gaps(self.conn, self.test_parquet, params)
        processed = process_gap_dataframe(df, sort_by="Gap Size")

        diffs = processed["diff"].tolist()
        self.assertEqual(diffs, sorted(diffs))

    def test_offset_and_limits(self):
        # Slice only rows 5 to 15 (11 rows)
        params = FilterParams(min_idx=5, max_idx=15, top_min=1, top_max=3, sort_by="Frequency")
        df = query_prime_gaps(self.conn, self.test_parquet, params)
        self.assertLessEqual(len(df), 3)
        self.assertEqual(df["frequency"].sum(), 11)


class TestAppModulesImport(unittest.TestCase):
    """Verifies that all application entrypoints and packages import cleanly."""

    def test_import_components(self):
        from jumpchamp_web import components
        self.assertTrue(hasattr(components, "render_gap_distribution_chart"))
        self.assertTrue(hasattr(components, "render_data_table"))
        self.assertTrue(hasattr(components, "render_math_definitions"))

    def test_import_runner(self):
        from jumpchamp_web import runner
        self.assertTrue(hasattr(runner, "run_app"))

    def test_import_facade(self):
        import app_common
        self.assertTrue(hasattr(app_common, "load_config"))
        self.assertTrue(hasattr(app_common, "get_db_connection"))


if __name__ == "__main__":
    unittest.main()
