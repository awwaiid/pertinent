use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <presentation.pin>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let content = fs::read_to_string(filename).expect("Error reading deck file");

    let deck = parser::parse_deck(&content)
        .expect("Error parsing deck file")
        .1;

    println!("Loaded deck with {} slides", deck.slides.len());
    if !deck.slides.is_empty() {
        println!("First slide content: {:?}", deck.slides[0].content);
    }

    // Get the directory of the presentation file for loading images
    let presentation_dir = Path::new(filename)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Pertinent - Presentation Viewer".to_string(),
                resolution: (1024, 768).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(SlideDeck {
            slides: deck.slides,
            global_options: deck.global_options,
        })
        .insert_resource(CurrentSlide(0))
        .insert_resource(PresentationDir(presentation_dir))
        .add_systems(Startup, setup)
        .add_systems(Update, keyboard_navigation)
        .add_systems(Update, update_slide_display)
        .run();
}

#[derive(Resource)]
struct SlideDeck {
    slides: Vec<parser::Slide>,
    global_options: Vec<String>,
}

#[derive(Resource)]
struct CurrentSlide(usize);

#[derive(Resource)]
struct PresentationDir(PathBuf);

#[derive(Component)]
struct SlideText;

#[derive(Component)]
struct SlideCounter;

#[derive(Component)]
struct BackgroundImage;

fn setup(mut commands: Commands) {
    println!("Setup function called");
    commands.spawn(Camera2d);

    // Spawn background image (initially invisible)
    commands.spawn((
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundImage,
        Visibility::Hidden,
        ZIndex(-1), // Put background behind everything
    ));

    // Spawn a full-screen container for centering
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ZIndex(0), // Put text on top
    ))
    .with_children(|parent| {
        // Spawn the main slide text entity (will be updated each frame)
        parent.spawn((
            Text::new(""),
            TextFont {
                font_size: 60.0,
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::new_with_justify(Justify::Center),
            SlideText,
        ));
    });

    // Spawn slide counter in bottom right
    commands.spawn((
        Text::new("1 / 1"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            bottom: Val::Px(20.0),
            ..default()
        },
        SlideCounter,
        ZIndex(1), // Counter on top of everything
    ));
}

fn keyboard_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut current_slide: ResMut<CurrentSlide>,
    deck: Res<SlideDeck>,
    mut exit: MessageWriter<AppExit>,
) {
    let max_slides = deck.slides.len();

    if keyboard.just_pressed(KeyCode::Escape) {
        println!("Escape pressed - exiting");
        exit.write(AppExit::Success);
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::Space) {
        println!("Right/Space pressed");
        if current_slide.0 < max_slides - 1 {
            current_slide.0 += 1;
            println!("Moving to slide {}", current_slide.0);
        }
    }

    if keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::Backspace) {
        if current_slide.0 > 0 {
            current_slide.0 -= 1;
        }
    }

    if keyboard.just_pressed(KeyCode::Home) {
        current_slide.0 = 0;
    }

    if keyboard.just_pressed(KeyCode::End) {
        current_slide.0 = max_slides.saturating_sub(1);
    }
}

fn update_slide_display(
    current_slide: Res<CurrentSlide>,
    deck: Res<SlideDeck>,
    presentation_dir: Res<PresentationDir>,
    text_query: Query<Entity, (With<SlideText>, Without<SlideCounter>)>,
    mut bg_query: Query<(&mut ImageNode, &mut Visibility), With<BackgroundImage>>,
    children_query: Query<&Children>,
    mut counter_query: Query<&mut Text, (With<SlideCounter>, Without<SlideText>)>,
    window_query: Query<&Window>,
    mut last_displayed: Local<Option<usize>>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    if deck.slides.is_empty() {
        println!("Deck is empty!");
        return;
    }

    // Only update if the slide has actually changed
    if let Some(last) = *last_displayed {
        if last == current_slide.0 {
            return;
        }
    }

    println!("Updating display for slide {}", current_slide.0);

    let slide_index = current_slide.0.min(deck.slides.len() - 1);
    let slide = &deck.slides[slide_index];

    println!("Slide content: {:?}", slide.content);

    // Update background image
    if let Ok((mut bg_image, mut bg_visibility)) = bg_query.single_mut() {
        if let Some(image_file) = get_background_image(&slide.options, &deck.global_options) {
            let image_path = presentation_dir.0.join(&image_file);
            println!("Loading background image: {:?}", image_path);

            // Load image directly from file system
            if image_path.exists() {
                if let Ok(image_bytes) = std::fs::read(&image_path) {
                    if let Ok(dynamic_image) = image::load_from_memory(&image_bytes) {
                        let bevy_image = Image::from_dynamic(
                            dynamic_image,
                            true,
                            RenderAssetUsages::RENDER_WORLD
                        );
                        bg_image.image = images.add(bevy_image);
                        *bg_visibility = Visibility::Visible;
                    } else {
                        println!("Failed to decode image: {:?}", image_path);
                        *bg_visibility = Visibility::Hidden;
                    }
                } else {
                    println!("Failed to read image file: {:?}", image_path);
                    *bg_visibility = Visibility::Hidden;
                }
            } else {
                println!("Image file not found: {:?}", image_path);
                *bg_visibility = Visibility::Hidden;
            }
        } else {
            *bg_visibility = Visibility::Hidden;
        }
    }

    // Update slide text
    if let Ok(text_entity) = text_query.single() {
        // Parse Pango markup
        let segments = parse_pango_markup(&slide.content);
        println!("Setting text with {} segments", segments.len());

        // Calculate base font size based on content length and window size
        let base_font_size = if let Ok(window) = window_query.single() {
            let window_width = window.resolution.width();
            let window_height = window.resolution.height();

            // Get available space
            let available_width = window_width * 0.9;
            let available_height = window_height * 0.8;

            // Estimate based on total text
            let total_text: String = segments.iter().map(|(s, _)| s.as_str()).collect();
            let char_count = total_text.chars().count();
            let line_count = total_text.lines().count().max(1);

            if char_count > 0 {
                let chars_per_line = (char_count as f32 / line_count as f32).max(1.0);
                let width_based = available_width / (chars_per_line * 0.6);
                let height_based = available_height / (line_count as f32 * 1.2);
                width_based.min(height_based).clamp(30.0, 120.0)
            } else {
                60.0
            }
        } else {
            60.0
        };

        // Clear old text spans
        if let Ok(children) = children_query.get(text_entity) {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        // Build styled text - spawn child TextSpan entities
        commands.entity(text_entity).with_children(|parent| {
            for (text_content, style) in segments {
                // Calculate final font size
                let font_size = style.font_size.unwrap_or(base_font_size) * style.font_size_mult;
                let color = style.color.unwrap_or(Color::WHITE);

                parent.spawn((
                    TextSpan::new(text_content),
                    TextFont {
                        font_size,
                        ..default()
                    },
                    TextColor(color),
                ));
            }
        });

        // Handle positioning options
        let _position = get_text_position(&slide.options, &deck.global_options);
        // TODO: Apply position to the node or text alignment
    } else {
        println!("ERROR: Could not find SlideText entity!");
    }

    // Update slide counter
    if let Ok(mut counter_text) = counter_query.single_mut() {
        **counter_text = format!("{} / {}", slide_index + 1, deck.slides.len());
    }

    // Remember what we just displayed
    *last_displayed = Some(slide_index);
}

// Parse Pango markup into styled text segments
fn parse_pango_markup(text: &str) -> Vec<(String, PangoStyle)> {
    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut style_stack: Vec<PangoStyle> = vec![PangoStyle::default()];

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
                        let color_offset = if tag[color_start..].starts_with("color=") { 6 } else { 11 };
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

fn parse_color(color_str: &str) -> Option<Color> {
    match color_str.to_lowercase().as_str() {
        "red" => Some(Color::srgb(1.0, 0.0, 0.0)),
        "orange" => Some(Color::srgb(1.0, 0.5, 0.0)),
        "yellow" => Some(Color::srgb(1.0, 1.0, 0.0)),
        "green" => Some(Color::srgb(0.0, 1.0, 0.0)),
        "blue" => Some(Color::srgb(0.0, 0.0, 1.0)),
        "purple" => Some(Color::srgb(0.5, 0.0, 0.5)),
        "white" => Some(Color::srgb(1.0, 1.0, 1.0)),
        "black" => Some(Color::srgb(0.0, 0.0, 0.0)),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct PangoStyle {
    font_size: Option<f32>,
    font_size_mult: f32,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color: Option<Color>,
}

impl Default for PangoStyle {
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

fn is_image_file(option: &str) -> bool {
    let lower = option.to_lowercase();
    lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".gif")
}

fn get_background_image(slide_options: &[String], global_options: &[String]) -> Option<String> {
    // Check slide options first, then global options
    slide_options.iter()
        .chain(global_options.iter())
        .find(|opt| is_image_file(opt))
        .map(|s| s.clone())
}

fn get_text_position(slide_options: &[String], global_options: &[String]) -> TextPosition {
    // Check slide options first, then global options
    for option in slide_options.iter().chain(global_options.iter()) {
        match option.as_str() {
            "top" => return TextPosition::Top,
            "center" => return TextPosition::Center,
            "bottom" => return TextPosition::Bottom,
            _ => continue,
        }
    }
    TextPosition::Center // Default
}

#[derive(Debug, Clone, Copy)]
enum TextPosition {
    Top,
    Center,
    Bottom,
}
