// Resolved slide structures - fully processed and ready for rendering

use std::path::PathBuf;
use super::types::{
    BackgroundSpec, FontStyle, FontWeight, RenderColor, TextAlign, TextDecoration, TextPosition,
};

/// A span of text with styling information
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub font_size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub decoration: TextDecoration,
    pub color: RenderColor,
}

impl TextSpan {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 60.0,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
            decoration: TextDecoration::default(),
            color: RenderColor::white(),
        }
    }
}

/// A fully resolved slide ready for rendering
#[derive(Debug, Clone)]
pub struct ResolvedSlide {
    pub background: BackgroundSpec,
    pub text_spans: Vec<TextSpan>,
    pub text_position: TextPosition,
    pub text_align: TextAlign,
    pub base_font_size: f32,
}

impl Default for ResolvedSlide {
    fn default() -> Self {
        Self {
            background: BackgroundSpec::default(),
            text_spans: Vec::new(),
            text_position: TextPosition::Center,
            text_align: TextAlign::Left,
            base_font_size: 60.0,
        }
    }
}

/// A fully resolved deck ready for rendering
#[derive(Debug, Clone)]
pub struct ResolvedDeck {
    pub slides: Vec<ResolvedSlide>,
    pub presentation_dir: PathBuf,
}
