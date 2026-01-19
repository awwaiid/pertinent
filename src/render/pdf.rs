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
pub struct PdfRenderer {
    doc: PdfDocumentReference,
    page_width_mm: f32,
    page_height_mm: f32,
    current_page: Option<(PdfPageIndex, PdfLayerIndex)>,
    font: IndirectFontRef,
    slide_count: usize,
}

impl PdfRenderer {
    /// Create a new PDF renderer
    pub fn new() -> Result<Self, PdfError> {
        // Use A4 landscape for 4:3 aspect ratio presentations
        let page_width_mm = 297.0;
        let page_height_mm = 210.0;

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
            current_page: Some((page1, layer1)),
            font,
            slide_count: 0,
        })
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
                // Fill the entire page with the background color using a polygon
                let points = vec![
                    (Point::new(Mm(0.0), Mm(0.0)), false),
                    (Point::new(Mm(self.page_width_mm), Mm(0.0)), false),
                    (Point::new(Mm(self.page_width_mm), Mm(self.page_height_mm)), false),
                    (Point::new(Mm(0.0), Mm(self.page_height_mm)), false),
                ];
                let polygon = Polygon {
                    rings: vec![points],
                    mode: PaintMode::Fill,
                    winding_order: WindingOrder::NonZero,
                };
                layer.set_fill_color(color_to_pdf(color));
                layer.add_polygon(polygon);
            }
            BackgroundSpec::Image { path, scale } => {
                let image_path = presentation_dir.join(path);
                if image_path.exists() {
                    if let Err(e) = self.render_background_image(layer, &image_path, *scale) {
                        eprintln!("Warning: Failed to render background image: {}", e);
                        self.draw_black_background(layer);
                    }
                } else {
                    self.draw_black_background(layer);
                }
            }
        }
        Ok(())
    }

    /// Draw a black background polygon
    fn draw_black_background(&self, layer: &PdfLayerReference) {
        let points = vec![
            (Point::new(Mm(0.0), Mm(0.0)), false),
            (Point::new(Mm(self.page_width_mm), Mm(0.0)), false),
            (Point::new(Mm(self.page_width_mm), Mm(self.page_height_mm)), false),
            (Point::new(Mm(0.0), Mm(self.page_height_mm)), false),
        ];
        let polygon = Polygon {
            rings: vec![points],
            mode: PaintMode::Fill,
            winding_order: WindingOrder::NonZero,
        };
        layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
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

        // Calculate text positioning
        let margin_mm = self.page_width_mm * 0.05; // 5% margin
        let available_width = self.page_width_mm - 2.0 * margin_mm;
        let available_height = self.page_height_mm - 2.0 * margin_mm;

        // Collect all text for measurement
        let full_text: String = slide.text_spans.iter().map(|s| s.text.as_str()).collect();
        let line_count = full_text.lines().count().max(1);

        // Approximate line height (in mm, assuming roughly 72 points per inch)
        let avg_font_size = if slide.text_spans.is_empty() {
            slide.base_font_size
        } else {
            slide.text_spans.iter().map(|s| s.font_size).sum::<f32>() / slide.text_spans.len() as f32
        };
        let line_height_mm = avg_font_size * 0.3528 * 1.2; // pts to mm with line spacing
        let total_text_height = line_count as f32 * line_height_mm;

        // Calculate starting position based on text position
        let (start_x, start_y) = calculate_text_start_position(
            slide.text_position,
            slide.text_align,
            margin_mm,
            available_width,
            available_height,
            total_text_height,
            self.page_height_mm,
        );
        // Set text color (use first span's color)
        if let Some(span) = slide.text_spans.first() {
            layer.set_fill_color(color_to_pdf(&span.color));
        }

        // Process text line by line for proper positioning
        let mut current_y = start_y;
        for line in full_text.lines() {
            if !line.is_empty() {
                layer.use_text(line, avg_font_size, Mm(start_x), Mm(current_y), &self.font);
            }
            current_y -= line_height_mm;
        }

        Ok(())
    }

    /// Finalize the PDF and save to file
    pub fn finalize(self, output_path: &Path) -> Result<(), PdfError> {
        let file = File::create(output_path)?;
        self.doc.save(&mut BufWriter::new(file))
            .map_err(|e| PdfError::PdfError(format!("Failed to save PDF: {:?}", e)))?;
        Ok(())
    }
}

/// Calculate starting position for text based on position and alignment
fn calculate_text_start_position(
    position: TextPosition,
    align: TextAlign,
    margin: f32,
    _available_width: f32,
    _available_height: f32,
    text_height: f32,
    page_height: f32,
) -> (f32, f32) {
    // X position based on alignment
    let x = match align {
        TextAlign::Left => margin,
        TextAlign::Center => margin, // Text cursor is at left; actual centering would need text measurement
        TextAlign::Right => margin, // Same limitation
    };

    // Y position based on text position (PDF y=0 is at bottom)
    let y = match position {
        TextPosition::Top | TextPosition::TopLeft | TextPosition::TopRight => {
            page_height - margin
        }
        TextPosition::Center | TextPosition::Left | TextPosition::Right => {
            page_height / 2.0 + text_height / 2.0
        }
        TextPosition::Bottom | TextPosition::BottomLeft | TextPosition::BottomRight => {
            margin + text_height
        }
    };

    (x, y)
}

/// Convert RenderColor to printpdf Color
fn color_to_pdf(color: &RenderColor) -> Color {
    Color::Rgb(Rgb::new(
        color.r as f32,
        color.g as f32,
        color.b as f32,
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
