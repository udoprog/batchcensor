use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pos {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub milliseconds: u32,
}

impl Pos {
    /// Convert into samples given a sample rate.
    pub fn as_samples(&self, sample_rate: u32) -> Option<u32> {
        let samples = 0u32
            .checked_add(self.hours.checked_mul(3600)?.checked_mul(sample_rate)?)?
            .checked_add(self.minutes.checked_mul(60)?.checked_mul(sample_rate)?)?
            .checked_add(self.seconds.checked_mul(sample_rate)?)?
            .checked_add(
                self.milliseconds
                    .checked_mul(sample_rate.checked_div(1000)?)?,
            )?;

        Some(samples)
    }

    /// Deserialize stringa as a position.
    pub fn parse(s: &str) -> Option<Pos> {
        let mut main = s.split(':');
        let last = main.next_back()?;
        let mut last = last.split(".");

        let seconds = match last.next()?.trim() {
            "" => 0,
            seconds => str::parse::<u32>(seconds).ok()?,
        };

        let milliseconds = str::parse::<u32>(last.next()?).ok()?;

        let minutes = main
            .next_back()
            .and_then(|s| str::parse::<u32>(s).ok())
            .unwrap_or_default();

        let hours = main
            .next_back()
            .and_then(|s| str::parse::<u32>(s).ok())
            .unwrap_or_default();

        Some(Pos {
            hours,
            minutes,
            seconds,
            milliseconds,
        })
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.hours > 0 {
            write!(fmt, "{:02}:", self.hours)?;
        }

        if self.minutes > 0 {
            write!(fmt, "{:02}:", self.hours)?;
        }

        if self.seconds > 0 {
            write!(fmt, "{:02}", self.seconds)?;
        }

        write!(fmt, ".{:03}", self.milliseconds)?;
        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for Pos {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        Pos::parse(&s).ok_or_else(|| <D::Error as serde::de::Error>::custom("bad position"))
    }
}

#[cfg(test)]
mod tests {
    use super::Pos;

    #[test]
    pub fn test() {
        assert_eq!(
            Pos {
                hours: 0,
                minutes: 0,
                seconds: 0,
                milliseconds: 123,
            },
            Pos::parse(".123").expect("bad position")
        );

        assert_eq!(
            Pos {
                hours: 0,
                minutes: 0,
                seconds: 42,
                milliseconds: 123,
            },
            Pos::parse("42.123").expect("bad position")
        );

        assert_eq!(
            Pos {
                hours: 0,
                minutes: 21,
                seconds: 42,
                milliseconds: 123,
            },
            Pos::parse("21:42.123").expect("bad position")
        );

        assert_eq!(
            Pos {
                hours: 12,
                minutes: 21,
                seconds: 42,
                milliseconds: 123,
            },
            Pos::parse("12:21:42.123").expect("bad position")
        );
    }
}
