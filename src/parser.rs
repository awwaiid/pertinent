// grammar Pinpoint::Grammar {
//   token TOP {
//     <slide-defaults>
//     <slide>*
//   }
//
//   token slide-defaults { .*? <?before ^^ "--" > }
//
//   token slide {
//     <header>
//     <content>
//   }
//
//   token header { ^^ "--" [ \h* '[' <setting> ']' ]* \h* \n }
//   # token slide-header { ^^ "--" (<slide-setting> \s*)* $$ \n }
//
//   token setting { <-[ \] ]>* }
//
//   token content { .*? <?before ^^ "--" > || .*$ }
// };

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{
        line_ending, multispace0, multispace1, not_line_ending,
    },
    combinator::rest,
    multi::{many0, many1},
    sequence::{delimited, preceded, terminated},
};

type SlideOptions = Vec<String>;

#[derive(Debug, PartialEq)]
pub struct Slide {
    pub options: SlideOptions,
    pub content: String,
}

#[derive(Debug, PartialEq)]
pub struct SlideDeck {
    pub global_options: SlideOptions,
    pub slides: Vec<Slide>,
}

// An individual "[blah]" option
fn option(input: &str) -> nom::IResult<&str, &str> {
    return preceded(multispace0, delimited(tag("["), take_until("]"), tag("]")))(input);
}

fn whitespace_or_comment(input: &str) -> nom::IResult<&str, &str> {
    let (input, _) = many0(alt((
        multispace1,
        preceded(tag("#"), terminated(not_line_ending, line_ending)),
    )))(input)?;
    return Ok((input, ""));
}

// The first slide has [foo] entries on individual lines
fn settings(input: &str) -> nom::IResult<&str, SlideOptions> {
    let (input, options) = terminated(
        many0(preceded(whitespace_or_comment, option)),
        whitespace_or_comment,
    )(input)?;

    let options = options.iter().map(|s| s.to_string()).collect();
    return Ok((input, options));
}

// Slide headers look like "-- [a] [b]\n"
fn header(input: &str) -> nom::IResult<&str, SlideOptions> {
    let option_list = many0(option);

    let (input, options) = terminated(
        preceded(many1(tag("-")), option_list),
        alt((whitespace_or_comment, line_ending)),
    )(input)?;

    let options = options.iter().map(|s| s.to_string()).collect();
    return Ok((input, options));
}

// Here we don't care about the content (yet)
// Instead parse until we get to the next slide divider
fn content(input: &str) -> nom::IResult<&str, &str> {
    let (input, content) = alt((terminated(take_until("\n-"), tag("\n")), rest))(input)?;

    return Ok((input, content));
}

// A whole slide has a header and content, ending on another slide or EOF
fn slide(input: &str) -> nom::IResult<&str, Slide> {
    let (input, options) = header(input)?;
    let (input, content) = content(input)?;
    return Ok((
        input,
        Slide {
            options: options,
            content: content.to_string(),
        },
    ));
}

fn slides(input: &str) -> nom::IResult<&str, Vec<Slide>> {
    return many0(slide)(input);
}

// A deck has slide-0 with global options and then a list of slides
pub fn parse_deck(input: &str) -> nom::IResult<&str, SlideDeck> {
    let (input, global_options) = settings(input)?;
    let (input, slides) = slides(input)?;
    return Ok((
        input,
        SlideDeck {
            global_options: global_options,
            slides: slides,
        },
    ));
}

#[cfg(test)]
mod tests {

    use super::*;
    use indoc::indoc;

    #[test]
    fn test_option() {
        assert_eq!(option("[foo]"), Ok(("", "foo")));
        assert_eq!(option("[bar]"), Ok(("", "bar")));
        assert_eq!(option("  [baz]"), Ok(("", "baz")));
        assert_eq!(option("[multiple words]"), Ok(("", "multiple words")));
        assert_eq!(option("[file.jpg]"), Ok(("", "file.jpg")));
        assert_eq!(option("[path/to/file.jpg]"), Ok(("", "path/to/file.jpg")));
        // Test with trailing content
        assert_eq!(option("[opt]remaining"), Ok(("remaining", "opt")));
    }

    #[test]
    fn test_whitespace_or_comment() {
        assert_eq!(whitespace_or_comment(""), Ok(("", "")));
        assert_eq!(whitespace_or_comment(" "), Ok(("", "")));
        assert_eq!(whitespace_or_comment("    "), Ok(("", "")));
        assert_eq!(whitespace_or_comment(" # hmm\n"), Ok(("", "")));
        assert_eq!(whitespace_or_comment("# hmm\n"), Ok(("", "")));
        assert_eq!(whitespace_or_comment("\n\n"), Ok(("", "")));
        assert_eq!(whitespace_or_comment("# comment\n  \n# another\n"), Ok(("", "")));
        assert_eq!(whitespace_or_comment("\t\n  # test\n\n"), Ok(("", "")));
    }

    #[test]
    fn test_settings() {
        assert_eq!(settings(""), Ok(("", vec![])));
        assert_eq!(
            settings("[option1]"),
            Ok(("", vec!["option1".to_string()]))
        );
        assert_eq!(
            settings("[option1]\n[option2]"),
            Ok(("", vec!["option1".to_string(), "option2".to_string()]))
        );
        assert_eq!(
            settings("# comment\n[opt1]\n  \n[opt2]\n"),
            Ok(("", vec!["opt1".to_string(), "opt2".to_string()]))
        );
        assert_eq!(
            settings("[background.jpg]\n[bottom]\n# style comment\n"),
            Ok(("", vec!["background.jpg".to_string(), "bottom".to_string()]))
        );
    }

    #[test]
    fn header_test() {
        assert_eq!(header("-\n"), Ok(("", vec![])));
        assert_eq!(header("--\n"), Ok(("", vec![])));
        assert_eq!(header("---------\n"), Ok(("", vec![])));
        assert_eq!(header("-- [a]\n"), Ok(("", vec!["a".to_string()])));
        assert_eq!(
            header("-- [a]  [c]\n"),
            Ok(("", vec!["a".to_string(), "c".to_string()]))
        );
        assert_eq!(
            header("-- [a]  [c] # Plus a comment\n"),
            Ok(("", vec!["a".to_string(), "c".to_string()]))
        );
    }

    #[test]
    fn content_test() {
        assert_eq!(
            content("stuff\nand\nthings\n--"),
            Ok(("--", "stuff\nand\nthings"))
        );
    }

    #[test]
    fn slide_test() {
        assert_eq!(
            slide("--\n"),
            Ok((
                "",
                Slide {
                    options: vec![],
                    content: "".to_string()
                }
            ))
        );
        assert_eq!(
            slide("-- [a]\nthings and stuff"),
            Ok((
                "",
                Slide {
                    options: vec!["a".to_string()],
                    content: "things and stuff".to_string()
                }
            ))
        );
        assert_eq!(
            slide("--- [opt1] [opt2]\nMultiple\nLines\nOf\nContent"),
            Ok((
                "",
                Slide {
                    options: vec!["opt1".to_string(), "opt2".to_string()],
                    content: "Multiple\nLines\nOf\nContent".to_string()
                }
            ))
        );
        assert_eq!(
            slide("-- # comment after header\nContent here"),
            Ok((
                "",
                Slide {
                    options: vec![],
                    content: "Content here".to_string()
                }
            ))
        );
    }

    #[test]
    fn slides_test() {
        assert_eq!(slides(""), Ok(("", vec![])));
        assert_eq!(
            slides("--\nhello"),
            Ok((
                "",
                vec![Slide {
                    options: vec![],
                    content: "hello".to_string()
                }]
            ))
        );
        assert_eq!(
            slides("--\nfirst\n--\nsecond"),
            Ok((
                "",
                vec![
                    Slide {
                        options: vec![],
                        content: "first".to_string()
                    },
                    Slide {
                        options: vec![],
                        content: "second".to_string()
                    }
                ]
            ))
        );
        assert_eq!(
            slides("-- [a]\nslide 1\n--- [b] [c]\nslide 2\n--\nslide 3"),
            Ok((
                "",
                vec![
                    Slide {
                        options: vec!["a".to_string()],
                        content: "slide 1".to_string()
                    },
                    Slide {
                        options: vec!["b".to_string(), "c".to_string()],
                        content: "slide 2".to_string()
                    },
                    Slide {
                        options: vec![],
                        content: "slide 3".to_string()
                    }
                ]
            ))
        );
    }

    #[test]
    fn deck_test() {
        assert_eq!(
            parse_deck(""),
            Ok((
                "",
                SlideDeck {
                    global_options: vec![],
                    slides: vec![]
                }
            ))
        );
        assert_eq!(
            parse_deck("--\nhello world"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec![],
                    slides: vec![Slide {
                        options: vec![],
                        content: "hello world".to_string()
                    }]
                }
            ))
        );
        assert_eq!(
            parse_deck("[a]\n--\nhello world"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec!["a".to_string()],
                    slides: vec![Slide {
                        options: vec![],
                        content: "hello world".to_string()
                    }]
                }
            ))
        );
        assert_eq!(
            parse_deck("[a]\n-- [b]\nhello world"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec!["a".to_string()],
                    slides: vec![Slide {
                        options: vec!["b".to_string()],
                        content: "hello world".to_string()
                    }]
                }
            ))
        );
        assert_eq!(
            parse_deck("[a]\n-- [b] [c] # fishies\nhello world\n-- [d]\nThis is dog\n"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec!["a".to_string()],
                    slides: vec![
                        Slide {
                            options: vec!["b".to_string(), "c".to_string()],
                            content: "hello world".to_string()
                        },
                        Slide {
                            options: vec!["d".to_string()],
                            content: "This is dog\n".to_string()
                        }
                    ]
                }
            ))
        );

        let example_deck = indoc! {"
            # the 0th \"slide\" provides default styling for the presentation
            [bottom]           # position of text
            [slide-bg.jpg]     # default slide background
            --- [black] [center] # override background and text position

            A presentation

            --------- # lines starting with hyphens separate slides

            The format is meant to be <u>simple</u>

            --- [ammo.jpg]  # override background

            • Bullet point lists through unicode
            • Evil, but sometimes needed
        "};

        assert_eq!(
            parse_deck(example_deck),
            Ok(("", SlideDeck {
                global_options: vec![
                    "bottom".to_string(),
                    "slide-bg.jpg".to_string()],
                slides: vec![
                    Slide {
                        options: vec!["black".to_string(), "center".to_string()],
                        content: "A presentation\n".to_string() },
                    Slide {
                        options: vec![],
                        content: "The format is meant to be <u>simple</u>\n".to_string() },
                    Slide {
                        options: vec!["ammo.jpg".to_string()],
                        content: "• Bullet point lists through unicode\n• Evil, but sometimes needed\n".to_string() }
                ] }))
        );
    }

    #[test]
    fn deck_with_multiple_global_options() {
        assert_eq!(
            parse_deck("[top]\n[left]\n[bg.jpg]\n--\nSlide 1\n--\nSlide 2"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec![
                        "top".to_string(),
                        "left".to_string(),
                        "bg.jpg".to_string()
                    ],
                    slides: vec![
                        Slide {
                            options: vec![],
                            content: "Slide 1".to_string()
                        },
                        Slide {
                            options: vec![],
                            content: "Slide 2".to_string()
                        }
                    ]
                }
            ))
        );
    }

    #[test]
    fn deck_with_comments() {
        let deck_with_comments = indoc! {"
            # Global settings
            [center]
            # Background image
            [background.jpg]

            # First slide
            -- [title.jpg] # Custom background
            Welcome to the presentation
            --
            Content here
        "};

        assert_eq!(
            parse_deck(deck_with_comments),
            Ok((
                "",
                SlideDeck {
                    global_options: vec!["center".to_string(), "background.jpg".to_string()],
                    slides: vec![
                        Slide {
                            options: vec!["title.jpg".to_string()],
                            content: "Welcome to the presentation".to_string()
                        },
                        Slide {
                            options: vec![],
                            content: "Content here\n".to_string()
                        }
                    ]
                }
            ))
        );
    }

    #[test]
    fn deck_with_varying_dash_counts() {
        assert_eq!(
            parse_deck("-\nSlide 1\n--\nSlide 2\n-------\nSlide 3"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec![],
                    slides: vec![
                        Slide {
                            options: vec![],
                            content: "Slide 1".to_string()
                        },
                        Slide {
                            options: vec![],
                            content: "Slide 2".to_string()
                        },
                        Slide {
                            options: vec![],
                            content: "Slide 3".to_string()
                        }
                    ]
                }
            ))
        );
    }

    #[test]
    fn deck_with_empty_and_content_slides() {
        assert_eq!(
            parse_deck("--\nFirst slide\n--\nSecond slide\n--\nThird slide"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec![],
                    slides: vec![
                        Slide {
                            options: vec![],
                            content: "First slide".to_string()
                        },
                        Slide {
                            options: vec![],
                            content: "Second slide".to_string()
                        },
                        Slide {
                            options: vec![],
                            content: "Third slide".to_string()
                        }
                    ]
                }
            ))
        );
    }

    #[test]
    fn deck_with_special_characters_in_content() {
        assert_eq!(
            parse_deck("--\n<b>Bold</b> & <i>italic</i>\n• Bullet\n→ Arrow"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec![],
                    slides: vec![Slide {
                        options: vec![],
                        content: "<b>Bold</b> & <i>italic</i>\n• Bullet\n→ Arrow".to_string()
                    }]
                }
            ))
        );
    }

    #[test]
    fn deck_with_special_characters_in_options() {
        assert_eq!(
            parse_deck("-- [file-name_v2.jpg] [setting:value]\nContent"),
            Ok((
                "",
                SlideDeck {
                    global_options: vec![],
                    slides: vec![Slide {
                        options: vec!["file-name_v2.jpg".to_string(), "setting:value".to_string()],
                        content: "Content".to_string()
                    }]
                }
            ))
        );
    }
}
