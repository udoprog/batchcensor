use crate::Range;

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Replace {
    pub kind: String,
    pub range: Range,
}
