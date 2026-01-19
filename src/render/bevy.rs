// Bevy-specific rendering code and conversions

use bevy::prelude::*;
use bevy::text::Justify;
use bevy::ui::{AlignItems, JustifyContent};
use super::types::{RenderColor, TextAlign, TextPosition};

/// Convert RenderColor to Bevy Color
impl From<RenderColor> for Color {
    fn from(color: RenderColor) -> Self {
        Color::srgba(color.r, color.g, color.b, color.a)
    }
}

impl From<&RenderColor> for Color {
    fn from(color: &RenderColor) -> Self {
        Color::srgba(color.r, color.g, color.b, color.a)
    }
}

/// Extension trait for TextPosition to get Bevy flexbox alignment
pub trait TextPositionExt {
    fn to_flexbox_alignment(&self) -> (JustifyContent, AlignItems);
}

impl TextPositionExt for TextPosition {
    fn to_flexbox_alignment(&self) -> (JustifyContent, AlignItems) {
        match self {
            TextPosition::Center => (JustifyContent::Center, AlignItems::Center),
            TextPosition::Top => (JustifyContent::Center, AlignItems::FlexStart),
            TextPosition::Bottom => (JustifyContent::Center, AlignItems::FlexEnd),
            TextPosition::Left => (JustifyContent::FlexStart, AlignItems::Center),
            TextPosition::Right => (JustifyContent::FlexEnd, AlignItems::Center),
            TextPosition::TopLeft => (JustifyContent::FlexStart, AlignItems::FlexStart),
            TextPosition::TopRight => (JustifyContent::FlexEnd, AlignItems::FlexStart),
            TextPosition::BottomLeft => (JustifyContent::FlexStart, AlignItems::FlexEnd),
            TextPosition::BottomRight => (JustifyContent::FlexEnd, AlignItems::FlexEnd),
        }
    }
}

/// Extension trait for TextAlign to get Bevy text justification
pub trait TextAlignExt {
    fn to_bevy_justify(&self) -> Justify;
}

impl TextAlignExt for TextAlign {
    fn to_bevy_justify(&self) -> Justify {
        match self {
            TextAlign::Left => Justify::Left,
            TextAlign::Center => Justify::Center,
            TextAlign::Right => Justify::Right,
        }
    }
}

// Component markers for Bevy ECS
#[derive(Component)]
pub struct SlideText;

#[derive(Component)]
pub struct SlideCounter;

#[derive(Component)]
pub struct BackgroundImage;

#[derive(Component)]
pub struct TextContainer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_color_to_bevy() {
        let color = RenderColor::rgb(1.0, 0.5, 0.0);
        let bevy_color: Color = color.into();
        // Just verify conversion doesn't panic
        assert!(matches!(bevy_color, Color::Srgba(_)));
    }

    #[test]
    fn test_text_position_to_flexbox() {
        let (j, a) = TextPosition::Center.to_flexbox_alignment();
        assert_eq!(j, JustifyContent::Center);
        assert_eq!(a, AlignItems::Center);

        let (j, a) = TextPosition::TopLeft.to_flexbox_alignment();
        assert_eq!(j, JustifyContent::FlexStart);
        assert_eq!(a, AlignItems::FlexStart);
    }

    #[test]
    fn test_text_align_to_bevy() {
        assert_eq!(TextAlign::Left.to_bevy_justify(), Justify::Left);
        assert_eq!(TextAlign::Center.to_bevy_justify(), Justify::Center);
        assert_eq!(TextAlign::Right.to_bevy_justify(), Justify::Right);
    }
}
