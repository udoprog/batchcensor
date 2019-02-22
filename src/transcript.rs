use crate::{Range, Replace};

/// A parsed stranscript.
#[derive(Debug, Clone)]
pub struct Transcript {
    pub text: String,
    pub replace: Vec<Replace>,
    /// Marked words without a timestamp.
    pub missing: Vec<String>,
}

impl Transcript {
    pub fn parse(text: &str) -> Result<Transcript, failure::Error> {
        let mut it = text.chars();

        let mut replace = Vec::new();
        let mut missing = Vec::new();

        while let Some(c) = it.next() {
            match c {
                '[' => {
                    let (word, range) = Self::parse_replace(&mut it)?;

                    match range {
                        Some(range) => {
                            replace.push(Replace { word, range });
                        }
                        None => {
                            missing.push(word);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Transcript {
            text: text.to_string(),
            replace,
            missing,
        })
    }

    /// Parse a single replacement: [word]{range}.
    pub fn parse_replace(
        it: &mut impl Iterator<Item = char>,
    ) -> Result<(String, Option<Range>), failure::Error> {
        let mut word = None;
        let mut buffer = String::new();

        while let Some(c) = it.next() {
            match c {
                ']' => {
                    word = Some(buffer);
                    break;
                }
                c => {
                    buffer.push(c);
                }
            }
        }

        let word = match word {
            Some(word) => word,
            None => {
                failure::bail!("missing word");
            }
        };

        let open = it.next();

        if open != Some('{') {
            return Ok((word, None));
        }

        let mut range = None;
        let mut buffer = String::new();

        while let Some(c) = it.next() {
            match c {
                '}' => {
                    range = Some(buffer);
                    break;
                }
                c => {
                    buffer.push(c);
                }
            }
        }

        let range = match range {
            Some(range) => range,
            None => {
                failure::bail!("missing range");
            }
        };

        let range = Range::parse(&range).ok_or_else(|| failure::format_err!("bad range"))?;

        Ok((word, Some(range)))
    }
}

impl<'de> serde::Deserialize<'de> for Transcript {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        Transcript::parse(&s).map_err(|e| <D::Error as serde::de::Error>::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::Transcript;
    use crate::{Range, Replace};

    #[test]
    pub fn test() -> Result<(), failure::Error> {
        let transcript = Transcript::parse("foo [bar]{01.123-$} [baz]{^-$}")?;

        let a = Replace {
            word: String::from("bar"),
            range: Range::parse("01.123-$").expect("valid range"),
        };

        assert_eq!(a, transcript.replace[0]);

        let b = Replace {
            word: String::from("baz"),
            range: Range::parse("^-$").expect("valid range"),
        };

        assert_eq!(b, transcript.replace[1]);
        Ok(())
    }
}
