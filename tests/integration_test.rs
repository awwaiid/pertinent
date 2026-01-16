use std::fs;

#[test]
fn test_parse_simple_deck() {
    let content =
        fs::read_to_string("tests/fixtures/simple.pin").expect("Failed to read test fixture");

    let result = parser::parse_deck(&content);
    assert!(result.is_ok());

    let (remaining, deck) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(deck.global_options.len(), 2);
    assert_eq!(deck.slides.len(), 2);
    assert_eq!(deck.global_options[0], "center");
    assert_eq!(deck.global_options[1], "background.jpg");
}

#[test]
fn test_parse_complex_deck() {
    let content =
        fs::read_to_string("tests/fixtures/complex.pin").expect("Failed to read test fixture");

    let result = parser::parse_deck(&content);
    assert!(result.is_ok());

    let (remaining, deck) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(deck.global_options.len(), 2);
    assert_eq!(deck.slides.len(), 3);

    // Check first slide
    assert_eq!(deck.slides[0].options.len(), 2);
    assert_eq!(deck.slides[0].options[0], "black");
    assert_eq!(deck.slides[0].options[1], "center");

    // Check second slide
    assert!(deck.slides[1].content.contains("Bullet Points"));

    // Check third slide
    assert_eq!(deck.slides[2].options.len(), 1);
    assert_eq!(deck.slides[2].options[0], "custom.jpg");
}

#[test]
fn test_parse_empty_deck() {
    let result = parser::parse_deck("");
    assert!(result.is_ok());

    let (remaining, deck) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(deck.global_options.len(), 0);
    assert_eq!(deck.slides.len(), 0);
}

#[test]
fn test_parse_deck_with_only_global_options() {
    let content = "[option1]\n[option2]\n";
    let result = parser::parse_deck(content);
    assert!(result.is_ok());

    let (remaining, deck) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(deck.global_options.len(), 2);
    assert_eq!(deck.slides.len(), 0);
}

#[test]
fn test_parse_deck_with_single_slide() {
    let content = "--\nSingle slide content";
    let result = parser::parse_deck(content);
    assert!(result.is_ok());

    let (remaining, deck) = result.unwrap();
    assert_eq!(remaining, "");
    assert_eq!(deck.slides.len(), 1);
    assert_eq!(deck.slides[0].content, "Single slide content");
}
