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
    character::complete::{multispace0, none_of, alphanumeric1},
    sequence::{ terminated, preceded, delimited },
    multi::fold_many0
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

// fn option(input: &str) -> nom::IResult<&str, &str> {
//     let (input, _) = tag("[")(input);
//     // let (input, stuff) = take_until("]")(input);
//     take_until("]")(input);
// }

fn header(input: &str) -> nom::IResult<&str, SlideOptions> {
    let (input, _) = tag("--")(input)?;
    let option_parser = preceded(multispace0, delimited(tag("["), alphanumeric1, tag("]")));
    let (input, options) =
      fold_many0(
          option_parser,
          Vec::new(),
          |mut acc: Vec<_>, item: &str| {
              acc.push(item.to_string());
              acc
          }
      )(input)?;

    return Ok((input, options));
}

#[test]
fn header_test() {
    assert_eq!(header("--\n"), Ok(("\n", [].to_vec())));
    assert_eq!(header("-- [a]\n"), Ok(("\n", ["a".to_string()].to_vec())));
}

fn slide(input: &str) -> nom::IResult<&str, Slide> {
    let (input, options) = header(input)?;
    return Ok((input, Slide {
        options: options,
        content: "".to_string()
    }));
}

fn parse_slides(i: &str) -> nom::IResult<&str, &str> {
  tag("hello")(i)
}

#[test]
fn parser_test() {

  assert_eq!(slide("--\n"), Ok(("\n",
        Slide {
            options: [].to_vec(),
            content: "".to_string()
        })));
  assert_eq!(slide("-- [a]\n"), Ok(("\n",
        Slide {
            options: ["a".to_string()].to_vec(),
            content: "".to_string()
        })));
}

#[test]
fn parser_test2() {
  assert_eq!(parse_slides("hello world"), Ok((" world", "hello")));
}

fn main() {
	println!("{:?}", parse_slides("hello"));
	println!("{:?}", parse_slides("hello world"));
	println!("{:?}", parse_slides("goodbye hello again"));
}
