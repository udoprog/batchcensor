#[derive(Debug, PartialEq, Eq)]
pub struct Pos {
    hours: u32,
    minutes: u32,
    seconds: u32,
    milliseconds: u32,
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
        let seconds = str::parse::<u32>(last.next()?).ok()?;
        let milliseconds = str::parse::<u32>(last.next()?).ok()?;

        let minutes = last
            .next()
            .and_then(|s| str::parse::<u32>(s).ok())
            .unwrap_or_default();

        let hours = last
            .next()
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

impl<'de> serde::Deserialize<'de> for Pos {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        Pos::parse(&s).ok_or_else(|| <D::Error as serde::de::Error>::custom("bad position"))
    }
}
