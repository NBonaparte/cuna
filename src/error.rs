use std::fmt;
use std::io;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Hash, Copy, Clone)]
pub enum InvalidArgument {
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("missing arguments")]
    MissingArgument,
    #[error("invalid id")]
    InvalidId,
}
#[derive(Debug, Error)]
pub enum ParseError {
    /// There is something wrong in the cue sheet
    #[error("SyntaxError: {0}")]
    SyntaxError(String),
    #[error("UnexpetedToken: {0}")]
    UnexpectedToken(String),
    #[error(transparent)]
    InvalidArgument(#[from] InvalidArgument),
    /// There is nothing to parse or reaches eof
    #[error("Nothing to parse or eof")]
    Empty,
    /// Fails to read a file
    #[error("IoError: {0}")]
    IoError(#[from] io::Error),
}
#[derive(Debug, Error)]
pub struct Error {
    #[source]
    error: ParseError,
    at: Option<usize>,
}

impl ParseError {
    pub fn syntax_error(content: impl fmt::Display, description: impl fmt::Display) -> Self {
        Self::err_msg(format!("{}: {}", content, description))
    }
    pub fn unexpected_token(msg: impl fmt::Display) -> Self {
        Self::UnexpectedToken(msg.to_string())
    }
    pub fn err_msg(msg: impl fmt::Display) -> Self {
        Self::SyntaxError(msg.to_string())
    }
}
impl From<ParseIntError> for ParseError {
    fn from(e: ParseIntError) -> Self {
        Self::err_msg(e)
    }
}
impl PartialEq for ParseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::InvalidArgument(t), Self::InvalidArgument(t2)) => t == t2,
            (Self::SyntaxError(msg), Self::SyntaxError(msg2)) => msg == msg2,
            (Self::UnexpectedToken(msg), Self::UnexpectedToken(msg2)) => msg == msg2,
            (Self::IoError(_), Self::IoError(_)) => true,
            _ => false,
        }
    }
}
impl Error {
    pub const EMPTY: Self = Self::from_parse_error(ParseError::Empty);

    pub const fn new(error: ParseError, at: usize) -> Self {
        Self {
            error,
            at: Some(at),
        }
    }
    pub const fn from_parse_error(error: ParseError) -> Self {
        Self { error, at: None }
    }
    pub const fn kind(&self) -> &ParseError {
        &self.error
    }
    #[deprecated(note = "Please use Error::pos() instead")]
    pub const fn at(&self) -> &Option<usize> {
        &self.at
    }
    pub const fn pos(&self) -> Option<usize> {
        self.at
    }
    pub(crate) fn set_pos(&mut self, pos: usize) {
        self.at.replace(pos);
    }
}
impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.at {
            Some(l) => write!(formatter, "{} at line {}", self.error, l),
            None => write!(formatter, "{}", self.error),
        }
    }
}
impl<E: Into<ParseError>> From<E> for Error {
    fn from(e: E) -> Self {
        Self::from_parse_error(e.into())
    }
}
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind()
    }
}
