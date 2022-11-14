use std::fmt::Display;

use error_stack::Context;

#[derive(Debug)]
pub struct ParserError;

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("failed to parse the pacbuild")
    }
}

impl Context for ParserError {}

// #[derive(Debug)]
// pub struct ValidationError;

// impl Display for ValidationError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str("failed to validate")
//     }
// }

// impl Context for ValidationError {}

#[derive(Debug)]
pub struct InvalidField;

impl Display for InvalidField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid field")
    }
}

impl Context for InvalidField {}

#[derive(Debug)]
pub struct MissingField;

impl Display for MissingField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("required field is missing")
    }
}

impl Context for MissingField {}
