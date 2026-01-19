use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod render;
use render::{
    ResolveConfig, resolve_deck,
    bevy::{BackgroundImage, SlideCounter, SlideText, TextContainer, TextAlignExt, TextPositionExt},
    pdf::export_to_pdf,
    resolver::{
        get_background_image, get_background_scale, get_command, get_text_align, get_text_position,
        has_no_markup, parse_pango_markup,
    },
    types::BackgroundScale,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <presentation.pin> [-o output.pdf]", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    // Check for export mode
    let export_pdf = if args.len() >= 4 && args[2] == "-o" {
        Some(args[3].clone())
    } else {
        None
    };

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

    // Handle PDF export mode using the new pure PDF renderer
    if let Some(output_path) = export_pdf {
        let config = ResolveConfig::default();

        let parsed_deck = parser::SlideDeck {
            slides: deck.slides,
            global_options: deck.global_options,
        };

        let resolved = resolve_deck(&parsed_deck, &presentation_dir, &config);

        match export_to_pdf(&resolved, Path::new(&output_path)) {
            Ok(()) => {
                println!("Successfully exported to {}", output_path);
            }
            Err(e) => {
                eprintln!("Failed to export PDF: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Run interactive Bevy app
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
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
        .add_systems(Update, update_slide_display);

    app.run();
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

    // Spawn a full-screen container for text positioning
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Percent(5.0)), // 5% padding like pinpoint
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        TextContainer,
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

    // Execute command associated with current slide on Enter
    if keyboard.just_pressed(KeyCode::Enter) {
        if !deck.slides.is_empty() {
            let slide_index = current_slide.0.min(deck.slides.len() - 1);
            let slide = &deck.slides[slide_index];
            if let Some(cmd) = get_command(&slide.options, &deck.global_options) {
                println!("Executing command: {}", cmd);
                // Execute command in background using shell
                std::thread::spawn(move || {
                    match std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .spawn()
                    {
                        Ok(_) => println!("Command started: {}", cmd),
                        Err(e) => eprintln!("Failed to execute command '{}': {}", cmd, e),
                    }
                });
            }
        }
    }
}

fn update_slide_display(
    current_slide: Res<CurrentSlide>,
    deck: Res<SlideDeck>,
    presentation_dir: Res<PresentationDir>,
    mut text_query: Query<(Entity, &mut TextLayout), (With<SlideText>, Without<SlideCounter>)>,
    mut bg_query: Query<(&mut ImageNode, &mut Visibility, &mut Node), (With<BackgroundImage>, Without<TextContainer>)>,
    mut container_query: Query<&mut Node, (With<TextContainer>, Without<BackgroundImage>)>,
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
    if let Ok((mut bg_image, mut bg_visibility, mut bg_node)) = bg_query.single_mut() {
        if let Some(image_file) = get_background_image(&slide.options, &deck.global_options) {
            let image_path = presentation_dir.0.join(&image_file);
            println!("Loading background image: {:?}", image_path);

            // Load image directly from file system
            if image_path.exists() {
                if let Ok(image_bytes) = std::fs::read(&image_path) {
                    if let Ok(dynamic_image) = ::image::load_from_memory(&image_bytes) {
                        let bevy_image = Image::from_dynamic(
                            dynamic_image,
                            true,
                            RenderAssetUsages::RENDER_WORLD
                        );
                        bg_image.image = images.add(bevy_image);
                        *bg_visibility = Visibility::Visible;

                        // Apply background scaling mode
                        let bg_scale = get_background_scale(&slide.options, &deck.global_options);
                        match bg_scale {
                            BackgroundScale::Fill => {
                                // Fill the screen, may crop (use min-width/min-height 100%)
                                bg_node.width = Val::Percent(100.0);
                                bg_node.height = Val::Percent(100.0);
                                bg_node.min_width = Val::Percent(100.0);
                                bg_node.min_height = Val::Percent(100.0);
                            }
                            BackgroundScale::Fit => {
                                // Fit within screen, maintain aspect ratio (default)
                                bg_node.width = Val::Percent(100.0);
                                bg_node.height = Val::Percent(100.0);
                                bg_node.min_width = Val::Auto;
                                bg_node.min_height = Val::Auto;
                            }
                            BackgroundScale::Stretch => {
                                // Stretch to fill exactly
                                bg_node.width = Val::Percent(100.0);
                                bg_node.height = Val::Percent(100.0);
                                bg_node.min_width = Val::Percent(100.0);
                                bg_node.min_height = Val::Percent(100.0);
                            }
                            BackgroundScale::Unscaled => {
                                // Keep original size, centered
                                bg_node.width = Val::Auto;
                                bg_node.height = Val::Auto;
                                bg_node.min_width = Val::Auto;
                                bg_node.min_height = Val::Auto;
                            }
                        }
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
    if let Ok((text_entity, mut text_layout)) = text_query.single_mut() {
        // Check if markup parsing should be disabled
        let no_markup = has_no_markup(&slide.options, &deck.global_options);

        // Parse Pango markup (or not, if no-markup is set)
        let segments = if no_markup {
            // Return raw text as single segment with default style
            vec![(slide.content.clone(), render::resolver::ParsedStyle::default())]
        } else {
            parse_pango_markup(&slide.content)
        };
        println!("Setting text with {} segments (no_markup={})", segments.len(), no_markup);

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

        // Apply text alignment
        let text_align = get_text_align(&slide.options, &deck.global_options);
        text_layout.justify = text_align.to_bevy_justify();

        // Build styled text - spawn child TextSpan entities
        commands.entity(text_entity).with_children(|parent| {
            for (text_content, style) in segments {
                // Calculate final font size
                let font_size = style.font_size.unwrap_or(base_font_size) * style.font_size_mult;
                let color = style.color
                    .map(|c| Color::from(&c))
                    .unwrap_or(Color::WHITE);

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
        let position = get_text_position(&slide.options, &deck.global_options);
        if let Ok(mut container_node) = container_query.single_mut() {
            let (justify, align) = position.to_flexbox_alignment();
            container_node.justify_content = justify;
            container_node.align_items = align;
        }
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

#[cfg(test)]
mod tests {
    use super::render::resolver::*;
    use super::render::types::*;

    #[test]
    fn test_parse_pango_markup_plain_text() {
        let segments = parse_pango_markup("Hello World");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "Hello World");
    }

    #[test]
    fn test_parse_pango_markup_bold() {
        let segments = parse_pango_markup("<b>bold text</b>");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "bold text");
        assert!(segments[0].1.bold);
    }

    #[test]
    fn test_parse_pango_markup_italic() {
        let segments = parse_pango_markup("<i>italic text</i>");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "italic text");
        assert!(segments[0].1.italic);
    }

    #[test]
    fn test_parse_pango_markup_nested() {
        let segments = parse_pango_markup("normal <b>bold <i>bold-italic</i> bold</b> normal");
        assert_eq!(segments.len(), 5);
        assert_eq!(segments[0].0, "normal ");
        assert!(!segments[0].1.bold);
        assert_eq!(segments[1].0, "bold ");
        assert!(segments[1].1.bold);
        assert!(!segments[1].1.italic);
        assert_eq!(segments[2].0, "bold-italic");
        assert!(segments[2].1.bold);
        assert!(segments[2].1.italic);
    }

    #[test]
    fn test_parse_pango_markup_span_color() {
        let segments = parse_pango_markup("<span color='red'>red text</span>");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "red text");
        assert!(segments[0].1.color.is_some());
    }

    #[test]
    fn test_parse_pango_markup_span_font() {
        let segments = parse_pango_markup("<span font='20'>small text</span>");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0, "small text");
        assert_eq!(segments[0].1.font_size, Some(20.0));
    }

    #[test]
    fn test_parse_color_named() {
        assert!(parse_color("red").is_some());
        assert!(parse_color("blue").is_some());
        assert!(parse_color("green").is_some());
        assert!(parse_color("white").is_some());
        assert!(parse_color("black").is_some());
    }

    #[test]
    fn test_parse_color_unknown() {
        assert!(parse_color("unknown").is_none());
    }

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
        assert!(matches!(get_text_position(&["top".to_string()], &[]), TextPosition::Top));
        assert!(matches!(get_text_position(&["center".to_string()], &[]), TextPosition::Center));
        assert!(matches!(get_text_position(&["bottom".to_string()], &[]), TextPosition::Bottom));
        assert!(matches!(get_text_position(&["left".to_string()], &[]), TextPosition::Left));
        assert!(matches!(get_text_position(&["right".to_string()], &[]), TextPosition::Right));
        assert!(matches!(get_text_position(&["top-left".to_string()], &[]), TextPosition::TopLeft));
        assert!(matches!(get_text_position(&["top-right".to_string()], &[]), TextPosition::TopRight));
        assert!(matches!(get_text_position(&["bottom-left".to_string()], &[]), TextPosition::BottomLeft));
        assert!(matches!(get_text_position(&["bottom-right".to_string()], &[]), TextPosition::BottomRight));
        assert!(matches!(get_text_position(&[], &["top".to_string()]), TextPosition::Top));
        assert!(matches!(get_text_position(&[], &[]), TextPosition::Center)); // default
    }

    #[test]
    fn test_get_text_align() {
        assert_eq!(get_text_align(&["text-align=left".to_string()], &[]), TextAlign::Left);
        assert_eq!(get_text_align(&["text-align=center".to_string()], &[]), TextAlign::Center);
        assert_eq!(get_text_align(&["text-align=right".to_string()], &[]), TextAlign::Right);
        assert_eq!(get_text_align(&[], &["text-align=center".to_string()]), TextAlign::Center);
        assert_eq!(get_text_align(&[], &[]), TextAlign::Left); // default
    }

    #[test]
    fn test_get_background_scale() {
        assert_eq!(get_background_scale(&["fill".to_string()], &[]), BackgroundScale::Fill);
        assert_eq!(get_background_scale(&["fit".to_string()], &[]), BackgroundScale::Fit);
        assert_eq!(get_background_scale(&["stretch".to_string()], &[]), BackgroundScale::Stretch);
        assert_eq!(get_background_scale(&["unscaled".to_string()], &[]), BackgroundScale::Unscaled);
        assert_eq!(get_background_scale(&[], &["fill".to_string()]), BackgroundScale::Fill);
        assert_eq!(get_background_scale(&[], &[]), BackgroundScale::Fit); // default
    }

    #[test]
    fn test_get_command() {
        assert_eq!(get_command(&["command=echo hello".to_string()], &[]), Some("echo hello".to_string()));
        assert_eq!(get_command(&["command=xeyes".to_string()], &[]), Some("xeyes".to_string()));
        assert_eq!(get_command(&[], &["command=default cmd".to_string()]), Some("default cmd".to_string()));
        assert_eq!(get_command(&[], &[]), None); // no command
        // Slide option overrides global
        assert_eq!(get_command(&["command=slide cmd".to_string()], &["command=global cmd".to_string()]), Some("slide cmd".to_string()));
    }

    #[test]
    fn test_has_no_markup() {
        assert!(has_no_markup(&["no-markup".to_string()], &[]));
        assert!(!has_no_markup(&["markup".to_string()], &[]));
        assert!(has_no_markup(&[], &["no-markup".to_string()]));
        assert!(!has_no_markup(&[], &[])); // default: use markup
        // Slide option overrides global
        assert!(!has_no_markup(&["markup".to_string()], &["no-markup".to_string()]));
    }
}
