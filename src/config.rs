//! Models for a single configuration file.

use crate::{Replace, Transcript};
use relative_path::{RelativePath, RelativePathBuf};
use std::slice;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
pub struct ReplaceFile {
    path: RelativePathBuf,
    /// Transcript of the recording.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    transcript: Option<Transcript>,
    /// Replacements. If empty, file is clean.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    replace: Vec<Replace>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Files {
    List(Vec<ReplaceFile>),
    Map(linked_hash_map::LinkedHashMap<RelativePathBuf, Transcript>),
    ListOfMaps(Vec<linked_hash_map::LinkedHashMap<RelativePathBuf, Transcript>>),
}

impl Files {
    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        match *self {
            Files::List(ref list) => list.is_empty(),
            Files::Map(ref map) => map.is_empty(),
            Files::ListOfMaps(ref list) => list.iter().all(|m| m.is_empty()),
        }
    }

    /// Iterate over all files.
    pub fn iter(&self) -> FilesIter<'_> {
        match *self {
            Files::List(ref list) => FilesIter::List(list.iter()),
            Files::Map(ref map) => FilesIter::Map(map.iter()),
            Files::ListOfMaps(ref list) => FilesIter::ListOfMaps {
                current: None,
                it: list.iter(),
            },
        }
    }

    /// Insert the given transcript for the specified path.
    fn insert(&mut self, path: RelativePathBuf, transcript: Transcript) {
        match *self {
            Files::List(ref mut list) => list.push(ReplaceFile {
                path,
                transcript: Some(transcript),
                replace: vec![],
            }),
            Files::Map(ref mut map) => {
                map.insert(path, transcript);
            }
            Files::ListOfMaps(ref mut list) => {
                let mut map = linked_hash_map::LinkedHashMap::new();
                map.insert(path, transcript);
                list.push(map);
            }
        }
    }
}

impl Default for Files {
    fn default() -> Self {
        Files::ListOfMaps(vec![])
    }
}

impl<'a> IntoIterator for &'a Files {
    type IntoIter = FilesIter<'a>;
    type Item = (&'a RelativePath, Vec<&'a Replace>, Option<&'a Transcript>);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over replacements.
pub enum FilesIter<'a> {
    List(slice::Iter<'a, ReplaceFile>),
    Map(linked_hash_map::Iter<'a, RelativePathBuf, Transcript>),
    ListOfMaps {
        current: Option<linked_hash_map::Iter<'a, RelativePathBuf, Transcript>>,
        it: slice::Iter<'a, linked_hash_map::LinkedHashMap<RelativePathBuf, Transcript>>,
    },
}

impl<'a> Iterator for FilesIter<'a> {
    type Item = (&'a RelativePath, Vec<&'a Replace>, Option<&'a Transcript>);

    fn next(&mut self) -> Option<Self::Item> {
        match *self {
            FilesIter::List(ref mut it) => {
                let ReplaceFile {
                    ref path,
                    ref transcript,
                    ref replace,
                } = it.next()?;
                Some((path, replace.iter().collect(), transcript.as_ref()))
            }
            FilesIter::Map(ref mut it) => {
                let (ref path, ref transcript) = it.next()?;
                Some((path, vec![], Some(transcript)))
            }
            FilesIter::ListOfMaps {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
pub struct ReplaceDir {
    pub path: RelativePathBuf,
    #[serde(default)]
    #[serde(rename = "file_prefix")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Files::is_empty")]
    pub files: Files,
}

impl ReplaceDir {
    /// Construct a new replacement for the given directory.
    pub fn new(path: RelativePathBuf) -> Self {
        ReplaceDir {
            path,
            prefix: None,
            suffix: None,
            file_extension: None,
            files: Files::List(vec![]),
        }
    }

    /// Insert the given file into the configuration.
    pub fn insert_file(
        &mut self,
        file_extension: Option<&str>,
        mut file: RelativePathBuf,
        transcript: Transcript,
    ) -> Result<(), failure::Error> {
        let file_extension = self
            .file_extension
            .as_ref()
            .map(|s| s.as_str())
            .or(file_extension);

        if let Some(e) = file_extension {
            if Some(e) != file.extension() {
                failure::bail!("extension does not match");
            }

            file = match file.file_stem() {
                Some(stem) => file.with_file_name(stem),
                None => file,
            };
        }

        if let Some(prefix) = self.prefix.as_ref() {
            let mut name = match file.file_name() {
                Some(name) => name,
                None => failure::bail!("expected file name"),
            };

            if !name.starts_with(prefix) {
                failure::bail!("bad prefix in file");
            }

            name = &name[prefix.len()..];
            file = file.with_file_name(name);
        }

        if let Some(suffix) = self.suffix.as_ref() {
            let mut name = match file.file_name() {
                Some(name) => name,
                None => failure::bail!("expected file name"),
            };

            if !name.ends_with(suffix) {
                failure::bail!("bad prefix in file");
            }

            name = &name[..(name.len() - suffix.len())];
            file = file.with_file_name(name);
        }

        self.files.insert(file, transcript);
        Ok(())
    }

    /// Test if the dir contains the given path.
    pub fn contains(&self, path: &RelativePath) -> bool {
        let stem = match path.file_stem() {
            Some(stem) => stem,
            None => return false,
        };

        if let Some(prefix) = self.prefix.as_ref() {
            if !stem.starts_with(prefix) {
                return false;
            }
        }

        if let Some(suffix) = self.suffix.as_ref() {
            if !stem.ends_with(suffix) {
                return false;
            }
        }

        if let Some(extension) = self.file_extension.as_ref() {
            match path.extension() {
                Some(e) if e == extension => {}
                _ => return false,
            }
        }

        true
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dirs: Vec<ReplaceDir>,
}

impl Config {
    /// Insert the given file.
    pub fn insert_file<'a>(
        &'a mut self,
        file_dir: &RelativePath,
        file: RelativePathBuf,
        transcript: Transcript,
    ) -> Result<(), failure::Error> {
        let mut found = None;

        for (i, dir) in self.dirs.iter().enumerate() {
            if dir.path == file_dir && dir.contains(&file) {
                found = Some(i);
                break;
            }
        }

        let i = match found {
            Some(i) => i,
            None => {
                let mut dir = ReplaceDir::new(file_dir.to_owned());
                dir.files = Files::ListOfMaps(vec![]);

                let len = self.dirs.len();
                self.dirs.push(dir);
                len
            }
        };

        let Config {
            ref mut dirs,
            ref file_extension,
            ..
        } = *self;

        dirs[i].insert_file(
            file_extension.as_ref().map(|s| s.as_str()),
            file,
            transcript,
        )?;
        Ok(())
    }

    /// Optimize configuration.
    pub fn optimize(&mut self) -> Result<(), failure::Error> {
        self.dirs.sort();
        Ok(())
    }
}
