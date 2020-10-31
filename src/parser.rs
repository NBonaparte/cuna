use crate::error::Error;
use crate::error::ParseError;
use crate::time::TimeStamp;
use crate::track::Index;
use crate::track::Track;
use crate::track::TrackInfo;
use crate::utils;
use crate::Cuna;
use std::fmt;
use std::iter::Enumerate;
use std::str::Lines;

macro_rules! fail {
    (token $token: expr) => {
        return Err($crate::error::ParseError::unexpected_token($token));
    };
    (syntax $cmd: expr, $msg: expr) => {
        return Err($crate::error::ParseError::syntax_error($cmd, $msg));
    };
    (skip_empty $e: expr) => {
        match $e {
            Ok(ok) => ok,
            Err(e) if e == $crate::error::Error::EMPTY => continue,
            Err(e) => return Err(e),
        }
    };
}
macro_rules! trim {
    ($s: expr) => {
        $s.trim_matches('"')
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command<'a> {
    Rem(&'a str),
    Title(&'a str),
    Performer(&'a str),
    Songwriter(&'a str),
    Catalog(u64),
    Cdtextfile(&'a str),
    File(&'a str, &'a str),
    Track(u8, &'a str),
    Index(u8, TimeStamp),
    Pregap(&'a str),
    Postgap(&'a str),
    Isrc(&'a str),
    Flags(&'a str),
}
#[derive(Debug, Clone)]
pub struct Parser<'a>(Enumerate<Lines<'a>>);

impl<'a> Command<'a> {
    pub fn new(s: &'a str) -> Result<Self, ParseError> {
        let s = match s.trim() {
            "" => return Err(ParseError::Empty),
            ts => ts,
        };
        let (content, command) = match utils::token(s) {
            Ok(ok) => ok,
            Err(_) => fail!(syntax s, "missing arguments"),
        };
        match command.to_ascii_lowercase().as_ref() {
            "rem" => Ok(Self::Rem(content)),
            "title" => Ok(Self::Title(trim!(content))),
            "performer" => Ok(Self::Performer(trim!(content))),
            "songwriter" => Ok(Self::Songwriter(trim!(content))),
            "catalog" => match utils::number(13)(content) {
                Ok((_, catalog)) => Ok(Self::Catalog(catalog)),
                Err(_) => fail!(syntax content, "invaild catalog"),
            },
            "cdtextfile" => Ok(Self::Cdtextfile(trim!(content))),
            "file" => match utils::quote_opt(content) {
                Ok((format, path)) => Ok(Self::File(trim!(path), format.trim())),
                Err(_) => fail!(syntax command, "missing arguments"),
            },
            "track" => match utils::token(content) {
                Ok((format, id)) => Ok(Self::Track(utils::number(2)(id)?.1, format)),
                Err(_) => fail!(syntax command, "missing arguments"),
            },
            "index" => match utils::token(content) {
                Ok((timestamp, id)) => Ok(Self::Index(utils::number(2)(id)?.1, timestamp.parse()?)),
                Err(_) => fail!(syntax command, "missing arguments"),
            },
            "pregap" => Ok(Self::Pregap(trim!(content))),
            "postgap" => Ok(Self::Postgap(trim!(content))),
            "isrc" => Ok(Self::Isrc(trim!(content))),
            "flag" => Ok(Self::Flags(trim!(content))),
            _ => Err(ParseError::unexpected_token(command)),
        }
    }
    pub fn parse(&self, sheet: &mut Cuna) -> Result<(), ParseError> {
        match *self {
            Self::Rem(s) => sheet.comments.push(s.to_owned()),
            Self::Title(s) => match sheet.last_track_mut() {
                Some(tk) => tk.push_title(s.to_owned()),
                None => sheet.header.push_title(s.to_owned()),
            },
            Self::Performer(s) => match sheet.last_track_mut() {
                Some(tk) => tk.push_performer(s.to_owned()),
                _ => sheet.header.push_performer(s.to_owned()),
            },
            Self::Songwriter(s) => match sheet.last_track_mut() {
                Some(tk) => tk.push_songwriter(s.to_owned()),
                _ => sheet.header.push_songwriter(s.to_owned()),
            },
            Self::Catalog(s) => {
                if sheet.header.catalog.is_none() {
                    sheet.header.catalog = Some(s);
                } else {
                    fail!(syntax self, "multiple `CATALOG` commands is not allowed")
                }
            }
            Self::Cdtextfile(s) => {
                sheet.header.set_cdtextfile(s.to_owned());
            }
            Self::File(name, format) => {
                sheet.push_file(TrackInfo::new(name.to_owned(), format.to_owned()));
            }
            Self::Track(id, format) => match sheet.last_file_mut() {
                Some(tk) => tk.push_track(Track::new_unchecked(id, format.to_owned())),
                None => fail!(syntax self, "Multiple `CATALOG` commands is not allowed"),
            },
            Self::Index(id, timestamp) => match sheet.last_track_mut() {
                Some(tk) if tk.postgap.is_none() => {
                    tk.push_index(Index::new_unchecked(id, timestamp))
                }
                Some(_) => fail!(syntax self, "Command `INDEX` should be before `POSTGAP`"),
                None => fail!(token "INDEX"),
            },
            Self::Pregap(timestamp) => match sheet.last_track_mut() {
                Some(tk) if tk.index.is_empty() && tk.pregap.is_none() => {
                    tk.set_pregep(timestamp.parse()?);
                }
                Some(tk) if !tk.index.is_empty() => {
                    fail!(syntax self, "Command `PREGAP` should be before `INDEX`")
                }
                Some(tk) if tk.pregap.is_some() => {
                    fail!(syntax self, "Multiple `PREGAP` commands are not allowed in one `TRACK` scope")
                }
                _ => fail!(token "PREGAP"),
            },
            Self::Postgap(timestamp) => match sheet.last_track_mut() {
                Some(tk) if tk.postgap.is_none() => {
                    tk.set_postgep(timestamp.parse()?);
                }
                Some(_) => {
                    fail!(syntax self, "Multiple `POSTGAP` commands are not allowed in one `TRACK` scope")
                }
                None => fail!(token "POSTGAP"),
            },
            Self::Isrc(s) => match sheet.last_track_mut() {
                Some(tk) if tk.isrc.is_none() => {
                    tk.set_isrc(s.to_owned());
                }
                Some(_) => {
                    fail!(syntax self, "Multiple `ISRC` commands are not allowed in one `TRACK` scope")
                }
                None => fail!(token "ISRC"),
            },
            Self::Flags(s) => match sheet.last_track_mut() {
                Some(tk) if tk.flags.is_none() => tk.push_flags(s.split(' ')),
                Some(_) => {
                    fail!(syntax self, "Multiple `FLAGS` commands are not allowed in one `TRACK` scope")
                }
                None => fail!(token "FLAGS"),
            },
        }
        Ok(())
    }
}
impl fmt::Display for Command<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = match *self {
            Self::Rem(c) => format!("REM {}", c),
            Self::Title(c) => format!(r#"TITLE "{}""#, c),
            Self::Performer(c) => format!(r#"PERFORMER "{}""#, c),
            Self::Songwriter(c) => format!(r#"SONGWRITER "{}""#, c),
            Self::Catalog(c) => format!("CATALOG {}", c),
            Self::Cdtextfile(c) => format!(r#"CDTEXTFILE "{}""#, c),
            Self::File(name, tp) => format!(r#"FILE "{}" {}"#, name, tp),
            Self::Track(id, format) => format!("TRACK {} {}", id, format),
            Self::Index(id, timestamp) => format!("INDEX {} {}", id, timestamp),
            Self::Pregap(c) => format!("PREGAP {}", c),
            Self::Postgap(c) => format!("POSTGAP {}", c),
            Self::Isrc(c) => format!("ISRC {}", c),
            Self::Flags(c) => format!("FLAG {}", c),
        };
        write!(formatter, "{}", command)
    }
}
impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self(s.lines().enumerate())
    }
    pub fn from_lines(lines: Lines<'a>) -> Self {
        Self(lines.enumerate())
    }
    pub fn set_lines(&mut self, lines: Lines<'a>) {
        self.0 = lines.enumerate();
    }
    pub fn current_line(&self) -> Option<&str> {
        self.0.clone().next().map(|(_, s)| s)
    }
    /// Parses one line and writes to state
    pub fn parse_next_line(&mut self, state: &mut Cuna) -> Result<(), Error> {
        self.parse_next_n_lines(1, state)
    }
    /// Parses n lines and writes to state
    /// Each line will be parsed and written to state until an Error is returned
    pub fn parse_next_n_lines(&mut self, n: usize, state: &mut Cuna) -> Result<(), Error> {
        for (at, line) in self.0.by_ref().take(n) {
            let to_error = |e| Error::new(e, at + 1);
            let command = fail!(skip_empty Command::new(line).map_err(to_error));
            command.parse(state).map_err(to_error)?;
        }
        Ok(())
    }
    /// Parses all the lines and writes to state
    /// Each line will be parsed and written to state until an Error is returned
    pub fn parse(self, state: &mut Cuna) -> Result<(), Error> {
        for (at, line) in self.0 {
            let to_error = |e| Error::new(e, at + 1);
            let command = fail!(skip_empty Command::new(line).map_err(to_error));
            command.parse(state).map_err(to_error)?;
        }
        Ok(())
    }
}
