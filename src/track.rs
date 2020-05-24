use anyhow::Error;
use anyhow::Result;
use nom::branch::alt;
use nom::sequence::tuple;
use nom::sequence::preceded;
use nom::bytes::complete::take_until;
use nom::bytes::complete::tag_no_case as tag;
use nom::combinator::rest;
use nom::Err as NomErr;
use std::collections::BTreeMap;
use std::str::FromStr;
use crate::time::Duration;
use crate::utils;

#[derive(Debug, Clone)]
pub struct Index {
    pub id: u8, // index id must between 1 and 99, this filed should be private
    pub begin_time: Duration,
}
#[derive(Debug, Clone, Default)]
pub struct Track {
    pub id: u8, // track-id must between 1 and 99
    pub format: String,
    pub index: Vec<Index>,
    pub pregap: Option<Duration>,
    pub postgap: Option<Duration>,
    pub title: Option<Vec<String>>,
    pub performer: Option<Vec<String>>,
    pub songwriter: Option<Vec<String>>,
    pub isrc: Option<String>,
    pub flags: Option<Vec<String>>
}
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub name: String,
    pub format: String,
    pub tracks: Vec<Track>,
}

impl Index {
    pub fn new(id: u8, begin_time: Duration) -> Result<Self> {
        if id <= 99 {
            Ok(Self { id, begin_time })
        } else {
            Err(anyhow::format_err!("index-id must be between 1 and 99"))
        }
    }
}
impl FromStr for Index {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, (id, duration)) = tuple((preceded(tag("INDEX "), utils::take_digit2), preceded(tag(" "), rest)))(s)
            .map_err(|_| anyhow::anyhow!("error"))?;
        Ok(Self { id: id.parse()?, begin_time: duration.parse()? })
    }
}
impl Track {
    pub fn new(id: u8, format: String) -> Result<Self> {
        if id <= 99 {
            Ok(Self { id, format, ..Self::default() })
        } else {
            Err(anyhow::format_err!("track-id must be between 1 and 99"))
        }
    }
    pub fn push_title(&mut self, title: String) {
        self.title.get_or_insert_with(|| Vec::with_capacity(1)).push(title)
    }
    pub fn push_performer(&mut self, performer: String) {
        self.performer.get_or_insert_with(|| Vec::with_capacity(1)).push(performer)
    }
    pub fn push_songwriter(&mut self, songwriter: String) {
        self.songwriter.get_or_insert_with(|| Vec::with_capacity(1)).push(songwriter)
    }
    pub fn push_index(&mut self, index: Index) {
        self.index.push(index)
    }
    pub fn set_pregep(&mut self, pregap: Duration) -> Option<Duration> {
        self.pregap.replace(pregap)
    }
    pub fn set_postgep(&mut self, postgap: Duration) -> Option<Duration> {
        self.postgap.replace(postgap)
    }
    pub fn set_isrc(&mut self, isrc: String) -> Option<String> {
        self.isrc.replace(isrc)
    }
    pub fn push_flag(&mut self, flag: String) {
        self.flags.get_or_insert_with(|| Vec::with_capacity(1)).push(flag)
    }
    pub fn push_flags<F, S>(&mut self, flags: F)
        where F: IntoIterator<Item = S>,
            S: Into<String>
    {
        self.flags.get_or_insert_with(|| Vec::new()).extend(flags.into_iter().map(Into::into))
    }
}
impl TrackInfo {
    pub const fn new(name: String, format: String) -> Self {
        Self::with_tracks(name, format, Vec::new())
    }
    pub const fn with_tracks(name: String, format: String, tracks: Vec<Track>) -> Self {
        Self { name, format, tracks }
    }
    pub fn last_track(&self) -> Option<&Track> {
        self.tracks.last()
    }
    pub fn last_track_mut(&mut self) -> Option<&mut Track> {
        self.tracks.last_mut()
    }
    pub fn push_track(&mut self , track: Track) {
        self.tracks.push(track)
    }
}

fn parse_track_lines<'a, I>(lines: I) -> Result<Track>
    where I: Iterator<Item = &'a str>
{
    let mut commands = BTreeMap::new();
    let mut indexs = Vec::new();
    let mut lines = lines.into_iter();
    let first_line = lines.next().unwrap();
    let (_, (id, track_type)) = preceded(tag("TRACK "), tuple((utils::take_digit2, preceded(tag(" "), rest))))(first_line.trim()).unwrap();
    for line in lines {
        match tags!("title", "performer", "songwriter", "isrc", "flags", "pregap", "postgap")(line.trim()) {
            Ok((content, command)) => match utils::quote_opt(content.trim()) {
                Ok((_, content)) => commands.entry(command.to_ascii_lowercase())
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(content),
                _ => return Err(anyhow::anyhow!("Unexcept error while parsing track")),
            },
            Err(NomErr::Error((content, _))) => indexs.push(content),
            _ => return Err(anyhow::anyhow!("Unexcept error while parsing track")),
        }
    }
    let [title, performer, songwriter, isrc, flags, pregap, postgap] = get!(commands,
        (title, performer, songwriter, isrc, flags, pregap, postgap));
    let index = indexs.into_iter()
        .map(FromStr::from_str)
        .collect::<Result<Vec<_>, _>>()?;
    let to_owned = |v: Vec<&str>| v.into_iter().map(|s| s.to_owned()).collect();
    let track = Track {
        id: id.parse()?,
        format: track_type.to_owned(),
        index: index,
        pregap: match pregap {
            Some(pregaps) if pregaps.len() == 1 => Some(pregaps[0].parse()?),
            Some(_) => return Err(anyhow::anyhow!("Too many pregaps")),
            None => None,
        },
        postgap: match postgap {
            Some(postgaps) if postgaps.len() == 1 => Some(postgaps[0].parse()?),
            Some(_) => return Err(anyhow::anyhow!("Too many postgap")),
            None => None,
        },
        title: title.map(to_owned),
        performer: performer.map(to_owned),
        songwriter: songwriter.map(to_owned),
        isrc: match isrc {
            Some(isrcs) if isrcs.len() == 1 => Some(isrcs[0].parse()?),
            Some(_) => return Err(anyhow::anyhow!("Too many isrcs")),
            None => None,
        },
        flags: flags.map(to_owned),
    };
    Ok(track)
}
pub(crate) fn parse_filetracks_lines<'a, I>(lines: I) -> Result<TrackInfo>
    where I: Iterator<Item = &'a str> + Clone
{
    let mut lines = lines.into_iter();
    let first_line = lines.next().unwrap();
    let (name, data_type) = preceded(tag("FILE "), alt((utils::quote_opt, take_until(" "))))(first_line)
        .map(|(dt, n)| (n, dt.trim_start()))
        .unwrap();
    let (_, tracks) = utils::scope(lines).unwrap();
    let tracks = tracks.into_iter()
        .map(IntoIterator::into_iter)
        .map(parse_track_lines)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TrackInfo { name: name.to_owned(), format: data_type.to_owned(), tracks })
}