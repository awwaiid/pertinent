use indoc::indoc;

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
    options: SlideOptions,
    content: String,
}

#[derive(Debug, PartialEq)]
pub struct SlideDeck {
    global_options: SlideOptions,
    slides: Vec<Slide>,
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

    #[test]
    fn test_whitespace_or_comment() {
        assert_eq!(whitespace_or_comment(""), Ok(("", "")));
        assert_eq!(whitespace_or_comment(" "), Ok(("", "")));
        assert_eq!(whitespace_or_comment("    "), Ok(("", "")));
        assert_eq!(whitespace_or_comment(" # hmm\n"), Ok(("", "")));
        assert_eq!(whitespace_or_comment("# hmm\n"), Ok(("", "")));
        assert_eq!(whitespace_or_comment("\n\n"), Ok(("", "")));
    }

    #[test]
    fn test_settings() {
        assert_eq!(settings(""), Ok(("", vec![])));
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
}
