//! Models for a single configuration file.

use crate::{Replace, Transcript};
use relative_path::{RelativePath, RelativePathBuf};
use std::slice;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReplaceFile {
    path: RelativePathBuf,
    /// Transcript of the recording.
    transcript: Option<Transcript>,
    /// Replacements. If empty, file is clean.
    #[serde(default)]
    replace: Vec<Replace>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum ReplaceListOrMap {
    List(Vec<ReplaceFile>),
    Map(linked_hash_map::LinkedHashMap<RelativePathBuf, Transcript>),
    ListOfMaps(Vec<linked_hash_map::LinkedHashMap<RelativePathBuf, Transcript>>),
}

impl ReplaceListOrMap {
    pub fn iter(&self) -> ReplaceListOrMapIter<'_> {
        match *self {
            ReplaceListOrMap::List(ref list) => ReplaceListOrMapIter::List(list.iter()),
            ReplaceListOrMap::Map(ref map) => ReplaceListOrMapIter::Map(map.iter()),
            ReplaceListOrMap::ListOfMaps(ref list) => ReplaceListOrMapIter::ListOfMaps {
                current: None,
                it: list.iter(),
            },
        }
    }
}

impl<'a> IntoIterator for &'a ReplaceListOrMap {
    type IntoIter = ReplaceListOrMapIter<'a>;
    type Item = (&'a RelativePath, Vec<&'a Replace>, Option<&'a Transcript>);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over replacements.
pub enum ReplaceListOrMapIter<'a> {
    List(slice::Iter<'a, ReplaceFile>),
    Map(linked_hash_map::Iter<'a, RelativePathBuf, Transcript>),
    ListOfMaps {
        current: Option<linked_hash_map::Iter<'a, RelativePathBuf, Transcript>>,
        it: slice::Iter<'a, linked_hash_map::LinkedHashMap<RelativePathBuf, Transcript>>,
    },
}

impl<'a> Iterator for ReplaceListOrMapIter<'a> {
    type Item = (&'a RelativePath, Vec<&'a Replace>, Option<&'a Transcript>);

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            ReplaceListOrMapIter::List(ref mut it) => {
                let ReplaceFile {
                    ref path,
                    ref transcript,
                    ref replace,
                } = it.next()?;
                Some((path, replace.iter().collect(), transcript.as_ref()))
            }
            ReplaceListOrMapIter::Map(ref mut it) => {
                let (ref path, ref transcript) = it.next()?;
                Some((path, vec![], Some(transcript)))
            }
            ReplaceListOrMapIter::ListOfMaps {
                ref mut current,
                ref mut it,
            } => loop {
                if let Some((ref path, ref transcript)) = current.as_mut().and_then(|it| it.next())
                {
                    return Some((path, vec![], Some(transcript)));
                }

                *current = match it.next() {
                    Some(n) => Some(n.iter()),
                    None => return None,
                }
            },
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ReplaceDir {
    pub path: RelativePathBuf,
    #[serde(default, rename = "file_prefix")]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub file_extension: Option<String>,
    pub files: ReplaceListOrMap,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub file_extension: Option<String>,
    #[serde(default)]
    pub dirs: Vec<ReplaceDir>,
}
