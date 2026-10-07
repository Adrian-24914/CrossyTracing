#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayPreset {
    Performance,
    High,
    Ultra,
}

const FLUID_RENDER_SIZES: [(usize, usize); 3] = [(640, 480), (680, 510), (720, 540)];
const BALANCED_RENDER_SIZES: [(usize, usize); 6] = [
    (640, 480),
    (680, 510),
    (720, 540),
    (768, 576),
    (792, 594),
    (816, 612),
];
const ULTRA_RENDER_SIZES: [(usize, usize); 10] = [
    (640, 480),
    (680, 510),
    (720, 540),
    (768, 576),
    (792, 594),
    (816, 612),
    (864, 648),
    (888, 666),
    (912, 684),
    (960, 720),
];

impl DisplayPreset {
    pub const ALL: [Self; 3] = [Self::Performance, Self::High, Self::Ultra];

    pub fn from_shortcut_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Performance => "Fluida",
            Self::High => "Equilibrada",
            Self::Ultra => "Ultra",
        }
    }

    pub const fn render_size(self) -> (usize, usize) {
        match self {
            Self::Performance => (720, 540),
            Self::High => (816, 612),
            Self::Ultra => (960, 720),
        }
    }

    pub const fn window_size(self) -> (usize, usize) {
        match self {
            Self::Performance => (960, 720),
            Self::High => (960, 720),
            Self::Ultra => (1280, 960),
        }
    }

    pub const fn adaptive_render_sizes(self) -> &'static [(usize, usize)] {
        match self {
            Self::Performance => &FLUID_RENDER_SIZES,
            Self::High => &BALANCED_RENDER_SIZES,
            Self::Ultra => &ULTRA_RENDER_SIZES,
        }
    }
}

pub const DEFAULT_DISPLAY_PRESET: DisplayPreset = DisplayPreset::High;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_keep_the_same_viewport_proportions() {
        for preset in DisplayPreset::ALL {
            let (width, height) = preset.render_size();
            assert_eq!(width * 3, height * 4);
        }
    }

    #[test]
    fn each_preset_increases_the_rendered_pixel_count() {
        let pixels: Vec<_> = DisplayPreset::ALL
            .into_iter()
            .map(|preset| {
                let (width, height) = preset.render_size();
                width * height
            })
            .collect();

        assert!(pixels.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn every_preset_keeps_the_window_large_without_stretching_the_scene() {
        for preset in DisplayPreset::ALL {
            let (render_width, render_height) = preset.render_size();
            let (window_width, window_height) = preset.window_size();

            assert!(window_width >= render_width);
            assert!(window_height >= render_height);
            assert_eq!(window_width * render_height, window_height * render_width);
        }
    }

    #[test]
    fn every_adaptive_ladder_ends_at_its_quality_ceiling() {
        for preset in DisplayPreset::ALL {
            assert_eq!(
                preset.adaptive_render_sizes().first().copied(),
                Some((640, 480))
            );
            assert_eq!(
                preset.adaptive_render_sizes().last().copied(),
                Some(preset.render_size())
            );
        }
    }
}
