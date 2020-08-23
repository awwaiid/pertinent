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
    character::complete::{multispace0, alphanumeric1, newline},
    sequence::{ terminated, preceded, delimited },
    multi::{ fold_many0, many0 },
    combinator::rest,
    branch::alt
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

// Slide headers look like "-- [a] [b]\n"
fn header(input: &str) -> nom::IResult<&str, SlideOptions> {
    let option = preceded(multispace0, delimited(tag("["), alphanumeric1, tag("]")));
    // let option_list = many0( option );
    let option_list =
        fold_many0(
            option,
            Vec::new(),
            |mut acc: Vec<_>, item: &str| {
                acc.push(item.to_string());
                acc
            });

    let (input, options) =
        terminated(
            preceded( tag("--"), option_list ),
            newline
        )(input)?;

    // let options = options.map(ToString::to_string).collect();
    return Ok((input, options));
}

#[test]
fn header_test() {
    assert_eq!(header("--\n"), Ok(("", [].to_vec())));
    assert_eq!(header("-- [a]\n"), Ok(("", ["a".to_string()].to_vec())));
    assert_eq!(header("-- [a]  [c]\n"), Ok(("", ["a".to_string(), "c".to_string()].to_vec())));
}

fn content(input: &str) -> nom::IResult<&str, &str> {
    let (input, content) = alt((
        terminated(take_until("\n--"), tag("\n")),
        rest
    ))(input)?;

    // let (input, content) = take_until("\n--")(input)?;
    // let (input, _) = newline(input)?;
    return Ok((input, content));
}


#[test]
fn content_test() {
    assert_eq!(content("stuff\nand\nthings\n--"), Ok(("--", "stuff\nand\nthings")));
}

fn slide(input: &str) -> nom::IResult<&str, Slide> {
    let (input, options) = header(input)?;
    let (input, content) = content(input)?;
    return Ok((input, Slide {
        options: options,
        content: content.to_string()
    }));
}

fn parse_slides(input: &str) -> nom::IResult<&str, Vec<Slide>> {
    return Ok(many0(slide)(input)?);
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
        parse_slides("--\nhello world"),
        Ok((
                "", vec![Slide { options: vec![], content: "hello world".to_string() }]
           )));
}

fn main() {
	println!("{:?}", parse_slides("hello"));
	println!("{:?}", parse_slides("hello world"));
	println!("{:?}", parse_slides("goodbye hello again"));
}
