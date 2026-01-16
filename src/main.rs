use bevy::prelude::*;
use bevy::app::AppExit;
use std::env;
use std::fs;

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

#[derive(Component)]
struct SlideText;

#[derive(Component)]
struct SlideCounter;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Spawn the main slide text entity (will be updated each frame)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 60.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(90.0),
            height: Val::Percent(80.0),
            left: Val::Percent(5.0),
            top: Val::Percent(10.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        SlideText,
    ));

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
        exit.write(AppExit::Success);
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::Space) {
        if current_slide.0 < max_slides - 1 {
            current_slide.0 += 1;
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
    mut text_query: Query<(&mut Text, &mut TextFont), (With<SlideText>, Without<SlideCounter>)>,
    mut counter_query: Query<&mut Text, (With<SlideCounter>, Without<SlideText>)>,
    window_query: Query<&Window>,
) {
    if !current_slide.is_changed() {
        return;
    }

    if deck.slides.is_empty() {
        return;
    }

    let slide_index = current_slide.0.min(deck.slides.len() - 1);
    let slide = &deck.slides[slide_index];

    // Update slide text
    if let Ok((mut text, mut font)) = text_query.single_mut() {
        // Strip simple Pango markup for now (basic support)
        let content = strip_pango_markup(&slide.content);

        // Calculate appropriate font size based on content length and window size
        if let Ok(window) = window_query.single() {
            let window_width = window.resolution.width();
            let window_height = window.resolution.height();

            // Get available space (90% of window)
            let available_width = window_width * 0.9;
            let available_height = window_height * 0.8;

            // Estimate font size based on content length
            let char_count = content.chars().count();
            let line_count = content.lines().count().max(1);

            // Calculate font size to fit content
            // Rough heuristic: estimate character width and line height
            let estimated_font_size = if char_count > 0 {
                let chars_per_line = (char_count as f32 / line_count as f32).max(1.0);
                let width_based = available_width / (chars_per_line * 0.6);
                let height_based = available_height / (line_count as f32 * 1.2);
                width_based.min(height_based).clamp(30.0, 120.0)
            } else {
                60.0
            };

            // Update text content and style
            **text = content;
            font.font_size = estimated_font_size;
        }

        // Handle positioning options
        let _position = get_text_position(&slide.options, &deck.global_options);
        // TODO: Apply position to the node or text alignment
    }

    // Update slide counter
    if let Ok(mut counter_text) = counter_query.single_mut() {
        **counter_text = format!("{} / {}", slide_index + 1, deck.slides.len());
    }
}

fn strip_pango_markup(text: &str) -> String {
    // Basic Pango markup removal - strips <b>, <i>, <u>, <span>, etc.
    let mut result = text.to_string();

    // Remove opening and closing tags
    let tags = ["b", "i", "u", "big", "small", "tt", "s", "sub", "sup"];
    for tag in &tags {
        result = result.replace(&format!("<{}>", tag), "");
        result = result.replace(&format!("</{}>", tag), "");
    }

    // Remove span tags (more complex, but basic removal)
    while let Some(start) = result.find("<span") {
        if let Some(end) = result[start..].find('>') {
            result.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }
    result = result.replace("</span>", "");

    result
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
