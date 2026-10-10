//! User preferences: theme, global limit and display toggles. Survive a reset.

pub const MIN_LIMIT: u64 = 1_000_000;
pub const MAX_LIMIT: u64 = 100_000_000_000;
pub const DEFAULT_LIMIT: u64 = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Dark,
    #[default]
    Light,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prefs {
    pub theme: ThemeMode,
    /// Largest selectable prime index `n`.
    pub max_prime_limit: u64,
    pub show_settings: bool,
    pub show_grid_lines: bool,
    pub show_pct_labels: bool,
    pub show_heatmap_meter: bool,
    pub show_bar_tooltip: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: ThemeMode::default(),
            max_prime_limit: DEFAULT_LIMIT,
            show_settings: false,
            show_grid_lines: true,
            show_pct_labels: true,
            show_heatmap_meter: true,
            show_bar_tooltip: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_mode_default_is_light() {
        assert_eq!(ThemeMode::default(), ThemeMode::Light);
    }

    #[test]
    fn test_defaults() {
        let p = Prefs::default();
        assert_eq!(p.max_prime_limit, 10_000_000);
        assert!(!p.show_bar_tooltip);
        assert!(p.show_grid_lines && p.show_pct_labels && p.show_heatmap_meter);
    }
}
