// Logic to resolve a SlideDeck into a ResolvedDeck

use std::path::{Path, PathBuf};
use parser::{Slide, SlideDeck};
use super::resolved::{ResolvedDeck, ResolvedSlide, TextSpan};
use super::types::{
    BackgroundScale, BackgroundSpec, FontStyle, FontWeight, RenderColor, RenderDimensions,
    TextAlign, TextDecoration, TextPosition,
};

/// Configuration for deck resolution
#[derive(Debug, Clone)]
pub struct ResolveConfig {
    pub dimensions: RenderDimensions,
}

impl Default for ResolveConfig {
    fn default() -> Self {
        Self {
            dimensions: RenderDimensions::default(),
        }
    }
}

/// Resolve a parsed SlideDeck into a ResolvedDeck ready for rendering
pub fn resolve_deck(
    deck: &SlideDeck,
    presentation_dir: &Path,
    config: &ResolveConfig,
) -> ResolvedDeck {
    let slides = deck
        .slides
        .iter()
        .map(|slide| resolve_slide(slide, &deck.global_options, config))
        .collect();

    ResolvedDeck {
        slides,
        presentation_dir: presentation_dir.to_path_buf(),
    }
}

/// Resolve a single slide
fn resolve_slide(
    slide: &Slide,
    global_options: &[String],
    config: &ResolveConfig,
) -> ResolvedSlide {
    let text_position = get_text_position(&slide.options, global_options);
    let text_align = get_text_align(&slide.options, global_options);
    let no_markup = has_no_markup(&slide.options, global_options);

    // Get background - check for image first, then color, then default to dark gray
    let background = if let Some(image_file) = get_background_image(&slide.options, global_options) {
        let scale = get_background_scale(&slide.options, global_options);
        BackgroundSpec::Image {
            path: PathBuf::from(image_file),
            scale,
        }
    } else if let Some(color) = get_background_color(&slide.options, global_options) {
        BackgroundSpec::SolidColor(color)
    } else {
        // Default to dark gray to match Bevy's default window color
        BackgroundSpec::SolidColor(RenderColor::dark_gray())
    };

    // Parse text into spans
    let text_spans = if no_markup {
        vec![TextSpan::new(&slide.content)]
    } else {
        let parsed = parse_pango_markup(&slide.content);
        let base_font_size = calculate_base_font_size(&slide.content, &config.dimensions);
        parsed
            .into_iter()
            .map(|(text, style)| style_to_text_span(text, style, base_font_size))
            .collect()
    };

    ResolvedSlide {
        background,
        text_spans,
        text_position,
        text_align,
    }
}

/// Calculate base font size based on content and dimensions
fn calculate_base_font_size(content: &str, dimensions: &RenderDimensions) -> f32 {
    let available_width = dimensions.width * 0.9;
    let available_height = dimensions.height * 0.8;

    let char_count = content.chars().count();
    let line_count = content.lines().count().max(1);

    if char_count > 0 {
        let chars_per_line = (char_count as f32 / line_count as f32).max(1.0);
        let width_based = available_width / (chars_per_line * 0.6);
        let height_based = available_height / (line_count as f32 * 1.2);
        width_based.min(height_based).clamp(30.0, 120.0)
    } else {
        60.0
    }
}

/// Intermediate parsed style from Pango markup
#[derive(Clone, Debug)]
pub struct ParsedStyle {
    pub font_size: Option<f32>,
    pub font_size_mult: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Option<RenderColor>,
}

impl Default for ParsedStyle {
    fn default() -> Self {
        Self {
            font_size: None,
            font_size_mult: 1.0,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: None,
        }
    }
}

/// Convert a parsed style to a TextSpan
fn style_to_text_span(text: String, style: ParsedStyle, base_font_size: f32) -> TextSpan {
    let font_size = style.font_size.unwrap_or(base_font_size) * style.font_size_mult;
    let weight = if style.bold { FontWeight::Bold } else { FontWeight::Normal };
    let font_style = if style.italic { FontStyle::Italic } else { FontStyle::Normal };
    let decoration = TextDecoration {
        underline: style.underline,
        strikethrough: style.strikethrough,
    };
    let color = style.color.unwrap_or(RenderColor::white());

    TextSpan {
        text,
        font_size,
        weight,
        style: font_style,
        decoration,
        color,
    }
}

/// Check if a string looks like an image file
pub fn is_image_file(option: &str) -> bool {
    let lower = option.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
}

/// Get the background image from options
pub fn get_background_image(slide_options: &[String], global_options: &[String]) -> Option<String> {
    slide_options
        .iter()
        .chain(global_options.iter())
        .find(|opt| is_image_file(opt))
        .cloned()
}

/// Get the background color from options (e.g., [black], [white], [red])
pub fn get_background_color(slide_options: &[String], global_options: &[String]) -> Option<RenderColor> {
    for option in slide_options.iter().chain(global_options.iter()) {
        if let Some(color) = parse_color(option) {
            return Some(color);
        }
    }
    None
}

/// Get the text position from options
pub fn get_text_position(slide_options: &[String], global_options: &[String]) -> TextPosition {
    for option in slide_options.iter().chain(global_options.iter()) {
        match option.as_str() {
            "center" => return TextPosition::Center,
            "top" => return TextPosition::Top,
            "bottom" => return TextPosition::Bottom,
            "left" => return TextPosition::Left,
            "right" => return TextPosition::Right,
            "top-left" => return TextPosition::TopLeft,
            "top-right" => return TextPosition::TopRight,
            "bottom-left" => return TextPosition::BottomLeft,
            "bottom-right" => return TextPosition::BottomRight,
            _ => continue,
        }
    }
    TextPosition::Center
}

/// Get the text alignment from options
pub fn get_text_align(slide_options: &[String], global_options: &[String]) -> TextAlign {
    for option in slide_options.iter().chain(global_options.iter()) {
        if option.starts_with("text-align=") {
            let value = &option[11..];
            return match value {
                "left" => TextAlign::Left,
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                _ => TextAlign::Left,
            };
        }
    }
    TextAlign::Left
}

/// Get the background scale from options
pub fn get_background_scale(slide_options: &[String], global_options: &[String]) -> BackgroundScale {
    for option in slide_options.iter().chain(global_options.iter()) {
        match option.as_str() {
            "fill" => return BackgroundScale::Fill,
            "fit" => return BackgroundScale::Fit,
            "stretch" => return BackgroundScale::Stretch,
            "unscaled" => return BackgroundScale::Unscaled,
            _ => continue,
        }
    }
    BackgroundScale::Fit
}

/// Get the command from options
pub fn get_command(slide_options: &[String], global_options: &[String]) -> Option<String> {
    for option in slide_options.iter().chain(global_options.iter()) {
        if option.starts_with("command=") {
            return Some(option[8..].to_string());
        }
    }
    None
}

/// Check if no-markup is enabled
pub fn has_no_markup(slide_options: &[String], global_options: &[String]) -> bool {
    for option in slide_options.iter().chain(global_options.iter()) {
        match option.as_str() {
            "no-markup" => return true,
            "markup" => return false,
            _ => continue,
        }
    }
    false
}

/// Parse a named color to RenderColor
pub fn parse_color(color_str: &str) -> Option<RenderColor> {
    match color_str.to_lowercase().as_str() {
        "red" => Some(RenderColor::rgb(1.0, 0.0, 0.0)),
        "orange" => Some(RenderColor::rgb(1.0, 0.5, 0.0)),
        "yellow" => Some(RenderColor::rgb(1.0, 1.0, 0.0)),
        "green" => Some(RenderColor::rgb(0.0, 1.0, 0.0)),
        "blue" => Some(RenderColor::rgb(0.0, 0.0, 1.0)),
        "purple" => Some(RenderColor::rgb(0.5, 0.0, 0.5)),
        "white" => Some(RenderColor::rgb(1.0, 1.0, 1.0)),
        "black" => Some(RenderColor::rgb(0.0, 0.0, 0.0)),
        _ => None,
    }
}

/// Extract a quoted value from a string like "'value'" or "\"value\""
fn extract_quoted_value(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let quote_char = s.chars().next().unwrap();
    if quote_char == '\'' || quote_char == '"' {
        if let Some(end) = s[1..].find(quote_char) {
            return Some(s[1..end + 1].to_string());
        }
    }
    None
}

/// Parse Pango markup into styled text segments
pub fn parse_pango_markup(text: &str) -> Vec<(String, ParsedStyle)> {
    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut style_stack: Vec<ParsedStyle> = vec![ParsedStyle::default()];

    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Save current text if any
            if !current_text.is_empty() {
                segments.push((current_text.clone(), style_stack.last().unwrap().clone()));
                current_text.clear();
            }

            // Parse tag
            let mut tag = String::new();
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '>' {
                    chars.next();
                    break;
                }
                tag.push(chars.next().unwrap());
            }

            // Process tag
            if tag.starts_with('/') {
                // Closing tag
                if style_stack.len() > 1 {
                    style_stack.pop();
                }
            } else {
                // Opening tag
                let mut new_style = style_stack.last().unwrap().clone();

                if tag == "b" {
                    new_style.bold = true;
                } else if tag == "i" {
                    new_style.italic = true;
                } else if tag == "u" {
                    new_style.underline = true;
                } else if tag == "s" {
                    new_style.strikethrough = true;
                } else if tag == "sup" {
                    new_style.font_size_mult *= 0.7;
                } else if tag == "sub" {
                    new_style.font_size_mult *= 0.7;
                } else if tag.starts_with("span") {
                    // Parse span attributes
                    if let Some(font_start) = tag.find("font=") {
                        let font_val = &tag[font_start + 5..];
                        if let Some(size_str) = extract_quoted_value(font_val) {
                            if let Ok(size) = size_str.parse::<f32>() {
                                new_style.font_size = Some(size);
                            }
                        }
                    }

                    if let Some(color_start) = tag.find("color=").or_else(|| tag.find("foreground=")) {
                        let color_offset = if tag[color_start..].starts_with("color=") {
                            6
                        } else {
                            11
                        };
                        let color_val = &tag[color_start + color_offset..];
                        if let Some(color_str) = extract_quoted_value(color_val) {
                            new_style.color = parse_color(&color_str);
                        }
                    }
                }

                style_stack.push(new_style);
            }
        } else {
            current_text.push(ch);
        }
    }

    // Save any remaining text
    if !current_text.is_empty() {
        segments.push((current_text, style_stack.last().unwrap().clone()));
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file("background.png"));
        assert!(is_image_file("photo.jpg"));
        assert!(is_image_file("image.jpeg"));
        assert!(is_image_file("anim.gif"));
        assert!(is_image_file("PHOTO.PNG"));
        assert!(!is_image_file("center"));
        assert!(!is_image_file("bottom"));
    }

    #[test]
    fn test_get_text_position() {
        assert_eq!(get_text_position(&["top".to_string()], &[]), TextPosition::Top);
        assert_eq!(get_text_position(&["center".to_string()], &[]), TextPosition::Center);
        assert_eq!(get_text_position(&["bottom".to_string()], &[]), TextPosition::Bottom);
        assert_eq!(get_text_position(&[], &["top".to_string()]), TextPosition::Top);
        assert_eq!(get_text_position(&[], &[]), TextPosition::Center);
    }

    #[test]
    fn test_get_text_align() {
        assert_eq!(get_text_align(&["text-align=left".to_string()], &[]), TextAlign::Left);
        assert_eq!(get_text_align(&["text-align=center".to_string()], &[]), TextAlign::Center);
        assert_eq!(get_text_align(&["text-align=right".to_string()], &[]), TextAlign::Right);
        assert_eq!(get_text_align(&[], &[]), TextAlign::Left);
    }

    #[test]
    fn test_get_background_scale() {
        assert_eq!(get_background_scale(&["fill".to_string()], &[]), BackgroundScale::Fill);
        assert_eq!(get_background_scale(&["fit".to_string()], &[]), BackgroundScale::Fit);
        assert_eq!(get_background_scale(&[], &[]), BackgroundScale::Fit);
    }

    #[test]
    fn test_get_command() {
        assert_eq!(get_command(&["command=echo hello".to_string()], &[]), Some("echo hello".to_string()));
        assert_eq!(get_command(&[], &[]), None);
    }

    #[test]
    fn test_has_no_markup() {
        assert!(has_no_markup(&["no-markup".to_string()], &[]));
        assert!(!has_no_markup(&["markup".to_string()], &[]));
        assert!(!has_no_markup(&[], &[]));
    }

    #[test]
    fn test_parse_color() {
        assert!(parse_color("red").is_some());
        assert!(parse_color("blue").is_some());
        assert!(parse_color("unknown").is_none());
    }

    #[test]
    fn test_parse_pango_markup_plain() {
        let segments = parse_pango_markup("Hello World");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "Hello World");
    }

    #[test]
    fn test_parse_pango_markup_bold() {
        let segments = parse_pango_markup("<b>bold</b>");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "bold");
        assert!(segments[0].1.bold);
    }

    #[test]
    fn test_parse_pango_markup_nested() {
        let segments = parse_pango_markup("normal <b>bold <i>both</i></b>");
        assert_eq!(segments.len(), 3);
        assert!(!segments[0].1.bold);
        assert!(segments[1].1.bold);
        assert!(!segments[1].1.italic);
        assert!(segments[2].1.bold);
        assert!(segments[2].1.italic);
    }
}
