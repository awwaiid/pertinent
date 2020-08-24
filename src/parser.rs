extern crate nom;

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
    bytes::complete::{ tag, take_until },
    character::complete::{multispace0, multispace1, alphanumeric1, line_ending, not_line_ending},
    sequence::{ terminated, preceded, delimited },
    multi::{ fold_many0, many0 },
    combinator::{rest, map_res},
    branch::{alt, permutation}
};


type SlideOptions = Vec<String>;

#[derive(Debug, PartialEq)]
pub struct Slide {
    options: SlideOptions,
    content: String
}

#[derive(Debug, PartialEq)]
pub struct SlideDeck {
    global_options: SlideOptions,
    slides: Vec<Slide>
}

fn option(input: &str) -> nom::IResult<&str, &str> {
    return preceded(multispace0, delimited(tag("["), alphanumeric1, tag("]")))(input);
}

fn whitespace_or_comment(input: &str) -> nom::IResult<&str, &str> {
    let (input, _) = many0(
        alt((
                multispace1,
                preceded(
                    tag("#"),
                    terminated( not_line_ending, line_ending))
                )))(input)?;
    return Ok((input, ""));
}

fn settings(input: &str) -> nom::IResult<&str, SlideOptions> {
    let (input, options) = terminated(
        many0(
            preceded( whitespace_or_comment, option )
        ),
        whitespace_or_comment
    )(input)?;

    let options = options.iter().map(|s| s.to_string()).collect();
    return Ok((input, options));
}

// Slide headers look like "-- [a] [b]\n"
fn header(input: &str) -> nom::IResult<&str, SlideOptions> {
    let option_list = many0( option );

    let (input, options) =
        terminated(
            preceded( tag("--"), option_list ),
            line_ending
            )(input)?;

    let options = options.iter().map(|s| s.to_string()).collect();
    return Ok((input, options));
}

fn content(input: &str) -> nom::IResult<&str, &str> {
    let (input, content) = alt((
            terminated(take_until("\n--"), tag("\n")),
            rest
            ))(input)?;

    // let (input, content) = take_until("\n--")(input)?;
    // let (input, _) = line_ending(input)?;
    return Ok((input, content));
}

fn slide(input: &str) -> nom::IResult<&str, Slide> {
    let (input, options) = header(input)?;
    let (input, content) = content(input)?;
    return Ok((input, Slide {
        options: options,
        content: content.to_string()
    }));
}

fn slides(input: &str) -> nom::IResult<&str, Vec<Slide>> {
    return many0(slide)(input);
}

pub fn parse_slides(input: &str) -> nom::IResult<&str, SlideDeck> {
    let (input, global_options) = settings(input)?;
    let (input, slides) = slides(input)?;
    return Ok((input,
            SlideDeck {
                global_options: global_options,
                slides: slides
            }
            ));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_whitespace_or_comment() {
        assert_eq!( whitespace_or_comment(""), Ok(("","")));
        assert_eq!( whitespace_or_comment(" "), Ok(("","")));
        assert_eq!( whitespace_or_comment("    "), Ok(("","")));
        assert_eq!( whitespace_or_comment(" # hmm\n"), Ok(("", "")));
        assert_eq!( whitespace_or_comment("# hmm\n"), Ok(("", "")));
        assert_eq!( whitespace_or_comment("\n\n"), Ok(("", "")));
    }

    #[test]
    fn test_settings() {
        assert_eq!( settings(""), Ok(("", vec![])));
    }

    #[test]
    fn header_test() {
        assert_eq!(header("--\n"), Ok(("", [].to_vec())));
        assert_eq!(header("-- [a]\n"), Ok(("", ["a".to_string()].to_vec())));
        assert_eq!(header("-- [a]  [c]\n"), Ok(("", ["a".to_string(), "c".to_string()].to_vec())));
    }


    #[test]
    fn content_test() {
        assert_eq!(content("stuff\nand\nthings\n--"), Ok(("--", "stuff\nand\nthings")));
    }

    #[test]
    fn parser_test() {

        assert_eq!(slide("--\n"), Ok(("",
                                      Slide {
                                          options: [].to_vec(),
                                          content: "".to_string()
                                      })));
        assert_eq!(slide("-- [a]\nthings and stuff"), Ok(("",
                                                          Slide {
                                                              options: ["a".to_string()].to_vec(),
                                                              content: "things and stuff".to_string()
                                                          })));
    }

    #[test]
    fn parser_test2() {
        assert_eq!(
            parse_slides(""),
            Ok((
                    "",
                    SlideDeck {
                        global_options: vec![],
                        slides: vec![]
                    }
               )));
        assert_eq!(
            parse_slides("--\nhello world"),
            Ok((
                    "",
                    SlideDeck {
                        global_options: vec![],
                        slides: vec![Slide { options: vec![], content: "hello world".to_string() }]
                    }
               )));
        assert_eq!(
            parse_slides("[a]\n--\nhello world"),
            Ok((
                    "",
                    SlideDeck {
                        global_options: vec!["a".to_string()],
                        slides: vec![Slide { options: vec![], content: "hello world".to_string() }]
                    }
               )));
    }

}

