//! Gap histograms: plain `Vec<u64>` where index = gap size, value = count.

/// Gaps at or beyond this size are not tracked.
pub const MAX_GAP: usize = 65_536;

pub type Histogram = Vec<u64>;

/// A zeroed histogram covering gaps `0..MAX_GAP`.
pub fn empty() -> Histogram {
    vec![0; MAX_GAP]
}

/// Counts one occurrence of `gap` (ignored if out of range).
pub fn record(hist: &mut [u64], gap: u64) {
    if let Some(slot) = hist.get_mut(gap as usize) {
        *slot += 1;
    }
}

/// Adds `from` into `into` element-wise (`from` may be shorter).
pub fn add(into: &mut [u64], from: &[u64]) {
    into.iter_mut().zip(from).for_each(|(a, b)| *a += b);
}

/// Length up to and including the last non-zero bin.
pub fn trimmed_len(hist: &[u64]) -> usize {
    hist.iter().rposition(|&c| c > 0).map_or(0, |i| i + 1)
}

/// Copy without trailing zero bins (saves memory when caching).
pub fn trimmed(hist: &[u64]) -> Histogram {
    hist[..trimmed_len(hist)].to_vec()
}

/// Non-zero `(gap, count)` pairs in ascending gap order.
pub fn entries(hist: &[u64]) -> impl Iterator<Item = (u64, u64)> + '_ {
    hist.iter()
        .enumerate()
        .filter(|&(_, &c)| c > 0)
        .map(|(g, &c)| (g as u64, c))
}

/// The `n` most frequent gaps (ties → smaller gap first), returned in ascending gap order.
pub fn top_gaps(hist: &[u64], n: usize) -> Vec<(u64, u64)> {
    let mut top: Vec<_> = entries(hist).collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    top.truncate(n);
    top.sort_by_key(|&(g, _)| g);
    top
}

/// Running (cumulative) totals of `frames`, each cut to `width` bins.
pub fn running_totals(frames: &[Histogram], width: usize) -> Vec<Histogram> {
    let mut acc = vec![0u64; width];
    frames
        .iter()
        .map(|frame| {
            add(&mut acc, &frame[..width.min(frame.len())]);
            acc.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_ignores_out_of_range() {
        let mut h = vec![0; 4];
        record(&mut h, 2);
        record(&mut h, 2);
        record(&mut h, 9);
        assert_eq!(h, vec![0, 0, 2, 0]);
    }

    #[test]
    fn test_trimmed_and_add() {
        let mut a = vec![1, 0, 3, 0, 0];
        assert_eq!(trimmed(&a), vec![1, 0, 3]);
        add(&mut a, &[1, 1]);
        assert_eq!(a, vec![2, 1, 3, 0, 0]);
        assert_eq!(trimmed_len(&[0, 0]), 0);
    }

    #[test]
    fn test_top_gaps_keeps_most_frequent_sorted_by_gap() {
        // gap: 0  1  2  3  4  5
        let h = vec![0, 5, 9, 5, 1, 7];
        // Top 3 by count: 9 (gap 2), 7 (gap 5), then the 5-tie resolves to the smaller gap 1.
        assert_eq!(top_gaps(&h, 3), vec![(1, 5), (2, 9), (5, 7)]);
        assert_eq!(top_gaps(&h, 10), vec![(1, 5), (2, 9), (3, 5), (4, 1), (5, 7)]);
    }

    #[test]
    fn test_running_totals() {
        let frames = vec![vec![1, 0, 2], vec![0, 1, 1], vec![1, 1, 0]];
        assert_eq!(running_totals(&frames, 2), vec![vec![1, 0], vec![1, 1], vec![2, 2]]);
    }
}
