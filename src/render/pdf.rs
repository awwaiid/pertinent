// Pure PDF renderer using printpdf for vector-based output

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder};
use super::resolved::{ResolvedDeck, ResolvedSlide};
use super::types::{BackgroundScale, BackgroundSpec, RenderColor, TextAlign, TextPosition};

/// Error type for PDF rendering
#[derive(Debug)]
pub enum PdfError {
    IoError(std::io::Error),
    PdfError(String),
    ImageError(String),
    FontError(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::IoError(e) => write!(f, "IO error: {}", e),
            PdfError::PdfError(s) => write!(f, "PDF error: {}", s),
            PdfError::ImageError(s) => write!(f, "Image error: {}", s),
            PdfError::FontError(s) => write!(f, "Font error: {}", s),
        }
    }
}

impl std::error::Error for PdfError {}

impl From<std::io::Error> for PdfError {
    fn from(e: std::io::Error) -> Self {
        PdfError::IoError(e)
    }
}

/// PDF renderer for batch slide export
/// Uses 4:3 aspect ratio to match the default 1024x768 window
pub struct PdfRenderer {
    doc: PdfDocumentReference,
    // Page dimensions in mm - using 4:3 aspect ratio
    // 270mm x 202.5mm gives us 4:3 and fits nicely on A4/Letter
    page_width_mm: f32,
    page_height_mm: f32,
    // Virtual pixel dimensions (matching Bevy window)
    pixel_width: f32,
    pixel_height: f32,
    current_page: Option<(PdfPageIndex, PdfLayerIndex)>,
    font: IndirectFontRef,
    slide_count: usize,
}

impl PdfRenderer {
    /// Create a new PDF renderer
    pub fn new() -> Result<Self, PdfError> {
        // Use 4:3 aspect ratio to match 1024x768 window
        // Scale to fit nicely: 270mm x 202.5mm (about 10.6" x 8")
        let page_width_mm = 270.0;
        let page_height_mm = 202.5;

        // Virtual pixel dimensions (same as Bevy window)
        let pixel_width = 1024.0;
        let pixel_height = 768.0;

        let (doc, page1, layer1) = PdfDocument::new(
            "Presentation",
            Mm(page_width_mm),
            Mm(page_height_mm),
            "Layer 1",
        );

        // Use a built-in PDF font (Helvetica)
        let font = doc.add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| PdfError::FontError(format!("Failed to add font: {:?}", e)))?;

        Ok(Self {
            doc,
            page_width_mm,
            page_height_mm,
            pixel_width,
            pixel_height,
            current_page: Some((page1, layer1)),
            font,
            slide_count: 0,
        })
    }

    /// Convert pixels to mm
    fn px_to_mm(&self, px: f32) -> f32 {
        px * (self.page_width_mm / self.pixel_width)
    }

    /// Render a single slide to the PDF
    pub fn render_slide(&mut self, slide: &ResolvedSlide, presentation_dir: &Path) -> Result<(), PdfError> {
        // Add a new page if this isn't the first slide
        if self.slide_count > 0 {
            let (page, layer) = self.doc.add_page(
                Mm(self.page_width_mm),
                Mm(self.page_height_mm),
                "Layer 1",
            );
            self.current_page = Some((page, layer));
        }

        let (page_idx, layer_idx) = self.current_page.unwrap();
        let layer = self.doc.get_page(page_idx).get_layer(layer_idx);

        // Render background
        self.render_background(&layer, &slide.background, presentation_dir)?;

        // Render text
        self.render_text(&layer, slide)?;

        self.slide_count += 1;
        Ok(())
    }

    /// Render the slide background
    fn render_background(
        &self,
        layer: &PdfLayerReference,
        background: &BackgroundSpec,
        presentation_dir: &Path,
    ) -> Result<(), PdfError> {
        match background {
            BackgroundSpec::SolidColor(color) => {
                self.draw_filled_rect(layer, 0.0, 0.0, self.page_width_mm, self.page_height_mm, color);
            }
            BackgroundSpec::Image { path, scale } => {
                let image_path = presentation_dir.join(path);
                if image_path.exists() {
                    if let Err(e) = self.render_background_image(layer, &image_path, *scale) {
                        eprintln!("Warning: Failed to render background image: {}", e);
                        self.draw_filled_rect(layer, 0.0, 0.0, self.page_width_mm, self.page_height_mm, &RenderColor::dark_gray());
                    }
                } else {
                    // Fall back to dark gray background if image not found
                    self.draw_filled_rect(layer, 0.0, 0.0, self.page_width_mm, self.page_height_mm, &RenderColor::dark_gray());
                }
            }
        }
        Ok(())
    }

    /// Draw a filled rectangle
    fn draw_filled_rect(&self, layer: &PdfLayerReference, x: f32, y: f32, width: f32, height: f32, color: &RenderColor) {
        let points = vec![
            (Point::new(Mm(x), Mm(y)), false),
            (Point::new(Mm(x + width), Mm(y)), false),
            (Point::new(Mm(x + width), Mm(y + height)), false),
            (Point::new(Mm(x), Mm(y + height)), false),
        ];
        let polygon = Polygon {
            rings: vec![points],
            mode: PaintMode::Fill,
            winding_order: WindingOrder::NonZero,
        };
        layer.set_fill_color(color_to_pdf(color));
        layer.add_polygon(polygon);
    }

    /// Render a background image
    fn render_background_image(
        &self,
        layer: &PdfLayerReference,
        image_path: &Path,
        scale: BackgroundScale,
    ) -> Result<(), PdfError> {
        // Load image using the image crate
        let dynamic_image = ::image::open(image_path)
            .map_err(|e| PdfError::ImageError(format!("Failed to load image: {}", e)))?;

        let rgb_image = dynamic_image.to_rgb8();
        let (width, height) = rgb_image.dimensions();

        // Create printpdf Image
        let image_data = rgb_image.into_raw();
        let pdf_image = Image::from(ImageXObject {
            width: Px(width as usize),
            height: Px(height as usize),
            color_space: ColorSpace::Rgb,
            bits_per_component: ColorBits::Bit8,
            interpolate: true,
            image_data,
            image_filter: None,
            smask: None,
            clipping_bbox: None,
        });

        // Calculate scale and position based on BackgroundScale
        let image_aspect = width as f32 / height as f32;
        let page_aspect = self.page_width_mm / self.page_height_mm;

        let (scaled_width, scaled_height, x, y) = match scale {
            BackgroundScale::Fit => {
                // Fit within page, may letterbox
                let (sw, sh) = if image_aspect > page_aspect {
                    (self.page_width_mm, self.page_width_mm / image_aspect)
                } else {
                    (self.page_height_mm * image_aspect, self.page_height_mm)
                };
                let x = (self.page_width_mm - sw) / 2.0;
                let y = (self.page_height_mm - sh) / 2.0;
                (sw, sh, x, y)
            }
            BackgroundScale::Fill => {
                // Fill page, may crop (we just scale to cover)
                let (sw, sh) = if image_aspect < page_aspect {
                    (self.page_width_mm, self.page_width_mm / image_aspect)
                } else {
                    (self.page_height_mm * image_aspect, self.page_height_mm)
                };
                let x = (self.page_width_mm - sw) / 2.0;
                let y = (self.page_height_mm - sh) / 2.0;
                (sw, sh, x, y)
            }
            BackgroundScale::Stretch => {
                // Stretch to fill exactly
                (self.page_width_mm, self.page_height_mm, 0.0, 0.0)
            }
            BackgroundScale::Unscaled => {
                // Keep original size, centered (convert pixels to mm at 96 DPI)
                let mm_per_px = 25.4 / 96.0;
                let sw = width as f32 * mm_per_px;
                let sh = height as f32 * mm_per_px;
                let x = (self.page_width_mm - sw) / 2.0;
                let y = (self.page_height_mm - sh) / 2.0;
                (sw, sh, x, y)
            }
        };

        // Add image to layer
        pdf_image.add_to_layer(layer.clone(), ImageTransform {
            translate_x: Some(Mm(x)),
            translate_y: Some(Mm(y)),
            scale_x: Some(scaled_width / width as f32),
            scale_y: Some(scaled_height / height as f32),
            ..Default::default()
        });

        Ok(())
    }

    /// Render the slide text
    fn render_text(&self, layer: &PdfLayerReference, slide: &ResolvedSlide) -> Result<(), PdfError> {
        if slide.text_spans.is_empty() {
            return Ok(());
        }

        // Collect all text for measurement
        let full_text: String = slide.text_spans.iter().map(|s| s.text.as_str()).collect();
        let full_text = full_text.trim(); // Trim whitespace like Bevy does

        if full_text.is_empty() {
            return Ok(());
        }

        // Calculate font size using the same algorithm as Bevy
        // Available space is 90% width, 80% height (matching Bevy's calculation)
        let available_width = self.pixel_width * 0.9;
        let available_height = self.pixel_height * 0.8;

        let char_count = full_text.chars().count();
        let line_count = full_text.lines().count().max(1);

        let font_size_px = if char_count > 0 {
            let chars_per_line = (char_count as f32 / line_count as f32).max(1.0);
            let width_based = available_width / (chars_per_line * 0.6);
            let height_based = available_height / (line_count as f32 * 1.2);
            width_based.min(height_based).clamp(30.0, 120.0)
        } else {
            60.0
        };

        // Convert font size from pixels to mm
        let font_size_mm = self.px_to_mm(font_size_px);
        // printpdf uses points (1 point = 1/72 inch = 0.3528mm)
        let font_size_pt = font_size_mm / 0.3528;

        let line_height_mm = font_size_mm * 1.2;

        // Calculate text block dimensions
        let lines: Vec<&str> = full_text.lines().collect();
        let text_block_height = lines.len() as f32 * line_height_mm;

        // Approximate character width for text measurement
        let approx_char_width = font_size_mm * 0.5;

        // Find the longest line for text block width calculation
        let max_line_chars = lines.iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as f32;
        let text_block_width = max_line_chars * approx_char_width;

        // 5% padding on each side (matching Bevy's padding: UiRect::all(Val::Percent(5.0)))
        let padding_x = self.page_width_mm * 0.05;
        let padding_y = self.page_height_mm * 0.05;
        let content_width = self.page_width_mm - 2.0 * padding_x;
        let content_height = self.page_height_mm - 2.0 * padding_y;

        // Calculate text block position based on TextPosition (matching Bevy's flexbox alignment)
        // This returns the top-left corner of the text block
        let (block_x, block_y) = self.calculate_text_block_position(
            slide.text_position,
            padding_x,
            padding_y,
            content_width,
            content_height,
            text_block_width,
            text_block_height,
            font_size_mm,
        );

        // Set text color (use first span's color, default white)
        let default_white = RenderColor::white();
        let text_color = slide.text_spans.first()
            .map(|s| &s.color)
            .unwrap_or(&default_white);
        layer.set_fill_color(color_to_pdf(text_color));

        // Render each line
        let mut current_y = block_y;
        for line in lines {
            if !line.is_empty() {
                // Calculate x position for this line based on text alignment within the block
                let line_width = line.chars().count() as f32 * approx_char_width;
                let line_x = match slide.text_align {
                    TextAlign::Left => block_x,
                    TextAlign::Center => block_x + (text_block_width - line_width) / 2.0,
                    TextAlign::Right => block_x + text_block_width - line_width,
                };

                layer.use_text(line, font_size_pt, Mm(line_x), Mm(current_y), &self.font);
            }
            current_y -= line_height_mm;
        }

        Ok(())
    }

    /// Calculate text block position based on TextPosition enum
    /// Returns (x, y) in mm where (x, y) is the top-left of the text block
    /// In PDF coordinates, y is the baseline of the first line
    fn calculate_text_block_position(
        &self,
        position: TextPosition,
        padding_x: f32,
        padding_y: f32,
        content_width: f32,
        _content_height: f32,
        text_block_width: f32,
        text_block_height: f32,
        font_size_mm: f32,
    ) -> (f32, f32) {
        // In PDF, y=0 is at the bottom, y increases upward
        // Text baseline is at the y coordinate

        // Horizontal position (x)
        let x = match position {
            TextPosition::Left | TextPosition::TopLeft | TextPosition::BottomLeft => {
                // Left aligned: start at left padding
                padding_x
            }
            TextPosition::Center | TextPosition::Top | TextPosition::Bottom => {
                // Horizontally centered: center the text block
                padding_x + (content_width - text_block_width) / 2.0
            }
            TextPosition::Right | TextPosition::TopRight | TextPosition::BottomRight => {
                // Right aligned: align text block to right edge
                padding_x + content_width - text_block_width
            }
        };

        // Vertical position (y) - this is the baseline of the first line
        let y = match position {
            TextPosition::Top | TextPosition::TopLeft | TextPosition::TopRight => {
                // Top: start at top of content area
                self.page_height_mm - padding_y - font_size_mm
            }
            TextPosition::Center | TextPosition::Left | TextPosition::Right => {
                // Vertically centered: center the text block
                let center_y = self.page_height_mm / 2.0;
                center_y + text_block_height / 2.0 - font_size_mm
            }
            TextPosition::Bottom | TextPosition::BottomLeft | TextPosition::BottomRight => {
                // Bottom: position at bottom of content area
                padding_y + text_block_height - font_size_mm
            }
        };

        (x, y)
    }

    /// Finalize the PDF and save to file
    pub fn finalize(self, output_path: &Path) -> Result<(), PdfError> {
        let file = File::create(output_path)?;
        self.doc.save(&mut BufWriter::new(file))
            .map_err(|e| PdfError::PdfError(format!("Failed to save PDF: {:?}", e)))?;
        Ok(())
    }
}

/// Convert RenderColor to printpdf Color
fn color_to_pdf(color: &RenderColor) -> Color {
    Color::Rgb(Rgb::new(
        color.r,
        color.g,
        color.b,
        None,
    ))
}

/// Export a resolved deck to PDF
pub fn export_to_pdf(
    deck: &ResolvedDeck,
    output_path: &Path,
) -> Result<(), PdfError> {
    let mut renderer = PdfRenderer::new()?;

    for slide in &deck.slides {
        renderer.render_slide(slide, &deck.presentation_dir)?;
    }

    renderer.finalize(output_path)?;
    println!("PDF export complete: {}", output_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_pdf() {
        let color = RenderColor::rgb(1.0, 0.5, 0.0);
        let pdf_color = color_to_pdf(&color);
        // Just verify conversion doesn't panic
        match pdf_color {
            Color::Rgb(_) => (),
            _ => panic!("Expected RGB color"),
        }
    }
}
