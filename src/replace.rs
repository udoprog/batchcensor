use crate::Range;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Replace {
    #[serde(rename = "kind")]
    pub word: String,
    pub range: Range,
}

impl fmt::Display for Replace {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "[{}]{{{}}}", self.word, self.range)
    }
}
