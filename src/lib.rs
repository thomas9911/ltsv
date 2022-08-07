//! [Labeled Tab-separated Values](http://ltsv.org/) parser in Rust and no-std compatible.
//!
//! ```rust
//! use ltsv::{tokenize, Pair};
//!
//! let data_iterator = tokenize("mylabel:1234\tmore_data:text");
//!
//! for line in data_iterator {
//!     for possible_pair in line {
//!         if let Ok(pair) = possible_pair {
//!             let data = Pair::from(pair);
//!             // do something with the data
//!             let label = data.label;
//!             let field = data.field;
//!         }
//!     }
//! }
//! ```
//!
//! The [`Data`] and [`Record`] structs both implement the Iterator behaviour, which allows to extract the data lazily.
//!
//! # Features
//!
//! ## std
//!
//! If the std feature is enabled there are some extra helper functions you can use.
//! For instance the [`parse`] function which early extracts all the data and puts it in a `Vec`
//!
//! ```rust
//! # #[cfg(feature = "std")] {
//! use ltsv::{parse, Pair};
//!
//! let out = parse("mylabel:1234\tmore_data:text");
//! let lines = out.unwrap();
//!
//! assert_eq!(Pair{label: "mylabel", field: "1234"}, lines[0][0]);
//! # }
//! ```
//!
//! # More examples
//!
//! print results back to ltsv:
//!
//! ```rust
//! use ltsv::Pair;
//!
//! let data = [Pair{label: "my_label", field: "testing"}, Pair{label: "my_label2", field: "testing"}];
//! let out = data.map(|x| x.to_string()).join(&ltsv::TAB.to_string());
//!
//! assert_eq!("my_label:testing\tmy_label2:testing", out);
//! ```
//!
//! Side note: This is not unicode aware, but I followed the original grammar when implement this

// grammar:
//
// ltsv = *(record NL) [record]
// record = [field *(TAB field)]
// field = label ":" field-value
// label = 1*lbyte
// field-value = *fbyte

// TAB = %x09 ;; \t
// NL = [%x0D] %x0A ;; \r\n
// lbyte = %x30-39 / %x41-5A / %x61-7A / "_" / "." / "-" ;; [0-9A-Za-z_.-]
// fbyte = %x01-08 / %x0B / %x0C / %x0E-FF

#![no_std]

#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "std")]
use std::vec::Vec;

pub const NEWLINE: char = '\n';
pub const TAB: char = '\t';
pub const SPLITTER: char = ':';

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    InvalidPair,
    InvalidLabel,
    InvalidField,
}

#[derive(Debug, PartialEq)]
pub struct Error<'a> {
    pub txt: &'a str,
    pub kind: ErrorKind,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

impl<'a> Error<'a> {
    pub fn invalid_pair(txt: &'a str) -> Error<'a> {
        Error {
            txt,
            kind: ErrorKind::InvalidPair,
            line: 0,
            start: 0,
            end: 0,
        }
    }

    pub fn invalid_label(txt: &'a str) -> Error<'a> {
        Error {
            txt,
            kind: ErrorKind::InvalidLabel,
            line: 0,
            start: 0,
            end: 0,
        }
    }

    pub fn invalid_field(txt: &'a str) -> Error<'a> {
        Error {
            txt,
            kind: ErrorKind::InvalidField,
            line: 0,
            start: 0,
            end: 0,
        }
    }

    pub fn set_line(&mut self, line: usize) {
        self.line = line;
    }

    pub fn set_span(&mut self, start: usize, end: usize) {
        self.start = start;
        self.end = end;
    }

    pub fn put_line(mut self, line: usize) -> Self {
        self.line = line;
        self
    }

    pub fn put_span(mut self, start: usize, end: usize) -> Self {
        self.start = start;
        self.end = end;
        self
    }
}

#[derive(Debug)]
pub struct Data<'a> {
    lines: core::str::Lines<'a>,
    pub current_line: usize,
}

impl<'a> Data<'a> {
    /// Runs the Iterator and allocates the data into a `Vec<Vec<_>>`.
    /// Short circuits on Error
    #[cfg(feature = "std")]
    pub fn run(self) -> Result<Vec<Vec<PairToken<'a>>>, Error<'a>> {
        let mut out = Vec::new();

        for line in self {
            let parsed_line: Result<Vec<PairToken<'a>>, Error<'a>> = line.collect();
            out.push(parsed_line?);
        }

        Ok(out)
    }
}

impl<'a> Iterator for Data<'a> {
    type Item = Record<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.lines.next()?;

        let record = Record {
            fields: line.split(TAB),
            current_line: self.current_line,
            current_pointer: 0,
        };

        self.current_line += 1;

        Some(record)
    }
}

#[derive(Debug)]
pub struct Record<'a> {
    fields: core::str::Split<'a, char>,
    pub current_line: usize,
    pub current_pointer: usize,
}

impl<'a> Iterator for Record<'a> {
    type Item = Result<PairToken<'a>, Error<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let pair = self.fields.next()?;
        // start + byte length of the field pair
        let end = self.current_pointer + pair.len();

        if let Some((label, field)) = pair.split_once(SPLITTER) {
            let pair = PairToken {
                label,
                field,
                line: self.current_line,
                start: self.current_pointer,
                end: end,
            };
            if let Err(e) = pair.validate() {
                return Some(Err(e));
            };

            // skip the tab character
            self.current_pointer = end + 1;

            return Some(Ok(pair));
        } else {
            return Some(Err(Error::invalid_pair(pair)
                .put_line(self.current_line)
                .put_span(self.current_pointer, end)));
        };
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Pair<'a> {
    pub label: &'a str,
    pub field: &'a str,
}

impl<'a> core::fmt::Display for Pair<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.label, self.field)
    }
}

impl<'a> From<PairToken<'a>> for Pair<'a> {
    fn from(input: PairToken<'a>) -> Pair<'a> {
        Pair {
            label: input.label,
            field: input.field,
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct PairToken<'a> {
    pub label: &'a str,
    pub field: &'a str,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

impl<'a> PairToken<'a> {
    pub fn new(label: &'a str, field: &'a str) -> PairToken<'a> {
        PairToken {
            label,
            field,
            ..Default::default()
        }
    }

    pub fn validate(&self) -> Result<(), Error<'a>> {
        self.validate_label()?;
        self.validate_field()?;
        Ok(())
    }

    fn validate_label(&self) -> Result<(), Error<'a>> {
        if self.label.as_bytes().iter().all(|b| match b {
            0x30..=0x39 => true,
            0x41..=0x5a => true,
            0x61..=0x7a => true,
            b'_' => true,
            b'.' => true,
            b'-' => true,
            _ => false,
        }) {
            Ok(())
        } else {
            Err(Error::invalid_label(self.label)
                .put_line(self.line)
                .put_span(self.start, self.start + self.label.len()))
        }
    }

    fn validate_field(&self) -> Result<(), Error<'a>> {
        if self.field.as_bytes().iter().all(|b| match b {
            0x01..=0x08 => true,
            0x0b => true,
            0x0c => true,
            0x0e..=0xff => true,
            _ => false,
        }) {
            Ok(())
        } else {
            Err(Error::invalid_field(self.field)
                .put_line(self.line)
                .put_span(self.start + self.label.len() + 1, self.end))
        }
    }
}

pub fn tokenize<'a>(input: &'a str) -> Data<'a> {
    Data {
        lines: input.lines(),
        current_line: 0,
    }
}

pub fn validate<'a>(input: &'a str) -> Result<(), Error<'a>> {
    for line in tokenize(input) {
        for field in line {
            field?;
        }
    }

    Ok(())
}

#[cfg(feature = "std")]
pub fn parse<'a>(input: &'a str) -> Result<Vec<Vec<Pair<'a>>>, Error<'a>> {
    let mut out = Vec::new();

    for line in tokenize(input) {
        let parsed_line: Result<Vec<Pair<'a>>, Error<'a>> = line.map(pair_from).collect();
        out.push(parsed_line?);
    }

    Ok(out)
}

#[cfg(feature = "std")]
fn pair_from<'a>(token: Result<PairToken<'a>, Error<'a>>) -> Result<Pair<'a>, Error<'a>> {
    Ok(Pair::from(token?))
}

#[cfg(all(test, feature = "std"))]
mod std_test {
    use super::*;
    use std::vec;

    #[test]
    fn parse_example() {
        let expected = Ok(vec![vec![
            Pair {
                label: "host",
                field: "127.0.0.1",
            },
            Pair {
                label: "ident",
                field: "-",
            },
            Pair {
                label: "user",
                field: "frank",
            },
            Pair {
                label: "time",
                field: "[10/Oct/2000:13:55:36 -0700]",
            },
            Pair {
                label: "req",
                field: "GET /apache_pb.gif HTTP/1.0",
            },
            Pair {
                label: "status",
                field: "200",
            },
            Pair {
                label: "size",
                field: "2326",
            },
            Pair {
                label: "referer",
                field: "http://www.example.com/start.html",
            },
            Pair {
                label: "ua",
                field: "Mozilla/4.08 [en] (Win98; I ;Nav)",
            },
        ]]);
        let out = parse("host:127.0.0.1\tident:-\tuser:frank\ttime:[10/Oct/2000:13:55:36 -0700]\treq:GET /apache_pb.gif HTTP/1.0\tstatus:200\tsize:2326\treferer:http://www.example.com/start.html\tua:Mozilla/4.08 [en] (Win98; I ;Nav)");

        assert_eq!(expected, out);
    }

    #[test]
    fn ignore_invalid_parts() {
        let expected = vec![
            Pair {
                label: "mylabel",
                field: "testing",
            },
            Pair {
                label: "more",
                field: "data",
            },
        ];
        let out =
            tokenize("!123:testing\tmylabel:testing\ttest\tinvalidfield:testing\rstuff\tmore:data");

        let mut fields = Vec::new();

        for line in out {
            for field in line {
                if let Ok(field) = field {
                    fields.push(Pair::from(field))
                }
            }
        }

        assert_eq!(fields, expected)
    }

    #[test]
    fn tokenize_test() {
        let expected = vec![
            vec![
                PairToken {
                    label: "mylabel",
                    field: "1",
                    line: 0,
                    start: 0,
                    end: 9,
                },
                PairToken {
                    label: "otherlabel",
                    field: "testing",
                    line: 0,
                    start: 10,
                    end: 28,
                },
            ],
            vec![
                PairToken {
                    label: "mylabel",
                    field: "2",
                    line: 1,
                    start: 0,
                    end: 9,
                },
                PairToken {
                    label: "otherlabel",
                    field: "more_testing",
                    line: 1,
                    start: 10,
                    end: 33,
                },
            ],
        ];

        let data = "mylabel:1\totherlabel:testing
mylabel:2\totherlabel:more_testing
";

        let out = tokenize(data).run();

        assert_eq!(Ok(expected), out);
    }

    #[test]
    fn tokenize_multiline() {
        let expected = vec![
            vec![PairToken {
                label: "mylabel",
                field: "1",
                line: 0,
                start: 0,
                end: 9,
            }],
            vec![PairToken {
                label: "mylabel",
                field: "2",
                line: 1,
                start: 0,
                end: 9,
            }],
            vec![PairToken {
                label: "mylabel",
                field: "3",
                line: 2,
                start: 0,
                end: 9,
            }],
            vec![PairToken {
                label: "mylabel",
                field: "4",
                line: 3,
                start: 0,
                end: 9,
            }],
            vec![PairToken {
                label: "mylabel",
                field: "5",
                line: 4,
                start: 0,
                end: 9,
            }],
        ];

        let data = "mylabel:1
mylabel:2
mylabel:3
mylabel:4
mylabel:5
";

        let out = tokenize(data).run();

        assert_eq!(Ok(expected), out);
    }
}

#[cfg(test)]
mod no_std_test {
    use super::*;

    #[test]
    fn tokenize_test() {
        let mut pairs: [Pair<'_>; 3] = [Pair::default(), Pair::default(), Pair::default()];

        let data = "mylabel1:1\tmylabel2:testing\tmylabel3:1234";

        let out = tokenize(data);

        let mut counter = 0;

        for line in out {
            for pair in line {
                if let Ok(x) = pair {
                    pairs[counter] = Pair::from(x);
                    counter += 1;
                }
            }
        }

        assert_eq!(
            pairs,
            [
                Pair {
                    label: "mylabel1",
                    field: "1"
                },
                Pair {
                    label: "mylabel2",
                    field: "testing"
                },
                Pair {
                    label: "mylabel3",
                    field: "1234"
                },
            ]
        )
    }

    #[test]
    fn invalid_label() {
        let expected = Err(Error::invalid_label("!123").put_span(0, 4));
        let out = validate("!123:testing");
        assert_eq!(expected, out);
        assert_eq!("!123", &"!123:testing"[0..4]);
    }

    #[test]
    fn invalid_field() {
        let expected = Err(Error::invalid_field("testing\rstuff").put_span(8, 21));
        let out = validate("mylabel:testing\rstuff");
        assert_eq!(expected, out);
        assert_eq!("testing\rstuff", &"mylabel:testing\rstuff"[8..21]);
    }

    #[test]
    fn invalid_pair() {
        let expected = Err(Error::invalid_pair("stuff").put_span(16, 21));
        let out = validate("mylabel:testing\tstuff");
        assert_eq!(expected, out);
        assert_eq!("stuff", &"mylabel:testing\tstuff"[16..21]);
    }
}
