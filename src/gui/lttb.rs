// ============================================================================
// LTTB (Largest-Triangle-Three-Buckets) Downsampling Algorithm
// ============================================================================

/// Downsample `data` to `threshold` points using the LTTB algorithm.
/// Input: slice of `[x, y]` pairs assumed sorted by x.
/// Output: Vec of `[f64; 2]` points.
pub fn downsample(data: &[[f64; 2]], threshold: usize) -> Vec<[f64; 2]> {
    let n = data.len();
    if n <= threshold || threshold <= 2 {
        return data.to_vec();
    }

    let mut sampled = Vec::with_capacity(threshold);
    sampled.push(data[0]);

    let bucket_size = (n - 2) as f64 / (threshold - 2) as f64;
    let mut a = 0usize;

    for i in 0..(threshold - 2) {
        let b_start = ((i as f64) * bucket_size + 1.0) as usize;
        let b_end = (((i + 1) as f64) * bucket_size + 1.0) as usize;
        let b_end = b_end.min(n - 1);

        let c_start = b_end;
        let c_end = (((i + 2) as f64) * bucket_size + 1.0) as usize;
        let c_end = c_end.min(n);

        let avg_c = if c_start < c_end {
            let sum_x: f64 = data[c_start..c_end].iter().map(|p| p[0]).sum();
            let sum_y: f64 = data[c_start..c_end].iter().map(|p| p[1]).sum();
            let cnt = (c_end - c_start) as f64;
            [sum_x / cnt, sum_y / cnt]
        } else {
            data[n - 1]
        };

        let (ax, ay) = (data[a][0], data[a][1]);
        let (cx, cy) = (avg_c[0], avg_c[1]);

        let mut max_area = -1.0f64;
        let mut max_idx = b_start;

        for (idx, p) in data[b_start..=b_end].iter().enumerate() {
            let area = ((ax - cx) * (p[1] - ay) - (ax - p[0]) * (cy - ay)).abs() * 0.5;
            if area > max_area {
                max_area = area;
                max_idx = b_start + idx;
            }
        }

        sampled.push(data[max_idx]);
        a = max_idx;
    }

    sampled.push(data[n - 1]);
    sampled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lttb_downsample_count() {
        let data: Vec<[f64; 2]> = (0..10_000).map(|i| [i as f64, (i as f64).sin()]).collect();
        let downsampled = downsample(&data, 500);
        assert_eq!(downsampled.len(), 500);
        assert_eq!(downsampled.first().unwrap(), &[0.0, 0.0]);
        assert_eq!(downsampled.last().unwrap(), &[9999.0, (9999.0f64).sin()]);
    }

    #[test]
    fn test_lttb_small_input() {
        let data = vec![[0.0, 1.0], [1.0, 2.0], [2.0, 3.0]];
        let downsampled = downsample(&data, 10);
        assert_eq!(downsampled.len(), 3);
    }
}
