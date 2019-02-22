use crate::Pos;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub start: Option<Pos>,
    pub end: Option<Pos>,
}

impl Range {
    /// Deserialize stringa as a position.
    pub fn parse(s: &str) -> Option<Range> {
        let mut main = s.split('-');
        let start = pos(main.next(), "^")?;
        let end = pos(main.next(), "$")?;

        return Some(Range { start, end });

        fn pos(pos: Option<&str>, term: &str) -> Option<Option<Pos>> {
            let pos = match pos {
                Some(pos) => pos,
                None => return None,
            };

            if pos == term {
                return Some(None);
            }

            let pos = Pos::parse(pos)?;
            Some(Some(pos))
        }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.start {
            Some(ref start) => start.fmt(fmt)?,
            None => "^".fmt(fmt)?,
        }

        write!(fmt, "-")?;

        match self.end {
            Some(ref end) => end.fmt(fmt)?,
            None => "^".fmt(fmt)?,
        }

        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for Range {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        Range::parse(&s).ok_or_else(|| <D::Error as serde::de::Error>::custom("bad position"))
    }
}
