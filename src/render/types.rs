// Renderer-agnostic types that don't depend on Bevy or any specific backend

use std::path::PathBuf;

/// A color represented as RGBA values (0.0 to 1.0)
#[derive(Clone, Debug, PartialEq)]
pub struct RenderColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl RenderColor {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn white() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }

    pub fn black() -> Self {
        Self::rgb(0.0, 0.0, 0.0)
    }

    /// Default dark gray background color (matches Bevy's default window color)
    /// Approximately RGB(50, 50, 56) - a pleasant dark charcoal gray
    pub fn dark_gray() -> Self {
        Self::rgb(0.196, 0.196, 0.22)
    }
}

impl Default for RenderColor {
    fn default() -> Self {
        Self::white()
    }
}

/// Text position on the slide
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextPosition {
    #[default]
    Center,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Text alignment within the text block
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// How background images should be scaled
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BackgroundScale {
    #[default]
    Fit,      // Fit within screen, may letterbox
    Fill,     // Fill screen, may crop
    Stretch,  // Stretch to fill, may distort
    Unscaled, // Keep original size
}

/// Font weight
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

/// Font style
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

/// Text decorations
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextDecoration {
    pub underline: bool,
    pub strikethrough: bool,
}

/// Dimensions for rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderDimensions {
    pub width: f32,
    pub height: f32,
}

impl Default for RenderDimensions {
    fn default() -> Self {
        Self {
            width: 1024.0,
            height: 768.0,
        }
    }
}

/// Background specification
#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundSpec {
    SolidColor(RenderColor),
    Image {
        path: PathBuf,
        scale: BackgroundScale,
    },
}

impl Default for BackgroundSpec {
    fn default() -> Self {
        Self::SolidColor(RenderColor::dark_gray())
    }
}
