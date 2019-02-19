use failure::ResultExt;
use relative_path::RelativePathBuf;
use std::{
    collections::BTreeSet,
    fs::File,
    path::{Path, PathBuf},
};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// A single task that can be executed.
pub enum Task<'a> {
    /// Regular processing with replacements.
    Process(PathBuf, PathBuf, &'a [Replace]),
    // Silent processing.
    ProcessSilent(PathBuf, PathBuf),
}

impl<'a> Task<'a> {
    fn run(self) -> Result<(), failure::Error> {
        match self {
            Task::Process(path, dest, replace) => {
                process_single(&path, &dest, replace).with_context(|_| {
                    failure::format_err!("failed to process: {}", path.display())
                })?;
            }
            Task::ProcessSilent(path, dest) => {
                process_silent(&path, &dest).with_context(|_| {
                    failure::format_err!("failed to process to silence: {}", path.display())
                })?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Replace {
    kind: String,
    range: Range,
}

#[derive(Debug, serde::Deserialize)]
pub struct ReplaceFile {
    path: RelativePathBuf,
    #[serde(default)]
    replace: Vec<Replace>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ReplaceDir {
    path: RelativePathBuf,
    #[serde(default)]
    file_prefix: Option<String>,
    #[serde(default)]
    file_extension: Option<String>,
    #[serde(default)]
    files: Vec<ReplaceFile>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    file_extension: Option<String>,
    #[serde(default)]
    dirs: Vec<ReplaceDir>,
}

#[derive(Debug)]
pub struct Pos {
    hours: u32,
    minutes: u32,
    seconds: u32,
    milliseconds: u32,
}

impl Pos {
    /// Convert into samples given a sample rate.
    pub fn as_samples(&self, sample_rate: u32) -> u32 {
        let mut samples = 0u32;
        samples += self.hours * 3600 * sample_rate;
        samples += self.minutes * 60 * sample_rate;
        samples += self.seconds * sample_rate;
        samples += self.milliseconds * (sample_rate / 1000);
        samples
    }
}

impl Pos {
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

#[derive(Debug)]
pub struct Range {
    pub start: Pos,
    pub end: Pos,
}

impl Range {
    /// Deserialize stringa as a position.
    pub fn parse(s: &str) -> Option<Range> {
        let mut main = s.split('-');
        let start = main.next().and_then(Pos::parse)?;
        let end = main.next().and_then(Pos::parse)?;

        Some(Range { start, end })
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

/// CLI options.
fn opts() -> clap::App<'static, 'static> {
    clap::App::new("Batch Censor")
        .version(VERSION)
        .author("John-John Tedro <udoprog@tedro.se>")
        .about("Batch censors a bunch of audio files.")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("<file>")
                .help("Configuration file to use.")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("config-dir")
                .short("d")
                .long("config-dir")
                .value_name("<dir>")
                .help("Configuration directory to use.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("root")
                .short("r")
                .long("root")
                .value_name("<dir>")
                .help("Root of project to process.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("list")
                .long("list")
                .help("List files which will be muted since they don't have a configuration."),
        )
}

/// Process a single file and apply all the specified replacements.
fn process_single(
    path: &Path,
    dest_path: &Path,
    replaces: &[Replace],
) -> Result<(), failure::Error> {
    if replaces.is_empty() {
        // Nothing to replace.
        return Ok(());
    }

    if dest_path.is_file() {
        std::fs::remove_file(dest_path)?;
    }

    std::fs::copy(path, dest_path)?;

    let r = File::open(path)?;
    let r = hound::WavReader::new(r)
        .with_context(|_| failure::format_err!("failed to open file: {}", path.display()))?;
    let s = r.spec();
    let duration = r.duration();

    let mut data = r.into_samples::<i16>().collect::<Result<Vec<i16>, _>>()?;

    for replace in replaces {
        let range = &replace.range;
        let mut start = range.start.as_samples(s.sample_rate) as usize;
        start *= s.channels as usize;

        let mut end = range.end.as_samples(s.sample_rate) as usize;
        end *= s.channels as usize;
        end = usize::min(end, duration as usize);

        let zeros = (start..end).map(|_| i16::default()).collect::<Vec<_>>();
        (&mut data[start..end]).copy_from_slice(&zeros);
    }

    let d = File::create(&dest_path)?;
    let mut w = hound::WavWriter::new(d, s)?;

    let mut writer = w.get_i16_writer(data.len() as u32);

    for d in data {
        writer.write_sample(d);
    }

    writer.flush()?;
    Ok(())
}

/// Replace the given file with silence.
fn process_silent(path: &Path, dest_path: &Path) -> Result<(), failure::Error> {
    if dest_path.is_file() {
        // Ignore files that already exist.
        return Ok(());
    }

    let r = File::open(path)?;
    let r = hound::WavReader::new(r)
        .with_context(|_| failure::format_err!("failed to open file: {}", path.display()))?;
    let s = r.spec();

    let d = File::create(&dest_path)?;
    let mut w = hound::WavWriter::new(d, s)?;

    let mut writer = w.get_i16_writer(r.duration());

    for _ in 0..(r.duration() * s.channels as u32) {
        writer.write_sample(0i16);
    }

    writer.flush()?;
    Ok(())
}

fn main() -> Result<(), failure::Error> {
    use rayon::prelude::*;

    let m = opts().get_matches();
    let list = m.is_present("list");

    let mut configs = Vec::new();
    configs.extend(
        m.values_of("config")
            .into_iter()
            .flat_map(|c| c)
            .map(PathBuf::from),
    );

    if let Some(config_dir) = m.value_of("config-dir") {
        for result in ignore::Walk::new(config_dir) {
            let result = result?;
            let path = result.path();

            if !path.is_file() {
                continue;
            }

            match path.extension().and_then(|s| s.to_str()) {
                Some("yml") => {}
                _ => continue,
            }

            configs.push(path.to_owned());
        }
    }

    let default_root = m.value_of("root").map(Path::new);

    let configs = configs
        .iter()
        .map(|path| {
            let f = File::open(path).with_context(|_| {
                failure::format_err!("could not open configuration: {}", path.display())
            })?;

            let config: Config = serde_yaml::from_reader(f)
                .with_context(|_| failure::format_err!("failed to parse: {}", path.display()))?;

            let root = match default_root {
                Some(root) => root,
                None => path.parent().ok_or_else(|| {
                    failure::format_err!("config does not have a parent directory")
                })?,
            };

            Ok((root, path, config))
        })
        .collect::<Result<Vec<_>, failure::Error>>()?;

    let mut tasks = Vec::new();

    for (root, config_path, config) in &configs {
        let output = root.join("output");

        for dir in &config.dirs {
            let root = dir.path.to_path(&root);

            if !root.is_dir() {
                failure::bail!("no such directory: {}", root.display());
            }

            let mut dest_root = output.to_owned();

            for c in dir.path.components() {
                dest_root.push(c.as_str());
            }

            if !dest_root.is_dir() {
                std::fs::create_dir_all(&dest_root)?;
            }

            let mut index = BTreeSet::new();

            for result in ignore::Walk::new(&root) {
                let path = result?.path().to_owned();

                match path.extension().and_then(|s| s.to_str()) {
                    Some("wav") => {}
                    _ => continue,
                }

                index.insert(path);
            }

            for f in &dir.files {
                let file_extension = dir
                    .file_extension
                    .as_ref()
                    .or(config.file_extension.as_ref());

                // temp storage for modified path.
                let mut replaced;
                let mut path = &f.path;

                if let Some(file_prefix) = dir.file_prefix.as_ref() {
                    let name = match path.file_name() {
                        Some(existing) => format!("{}{}", file_prefix, existing),
                        None => file_prefix.to_string(),
                    };

                    let new_path = path.with_file_name(name);
                    replaced = None;
                    path = replaced.get_or_insert(new_path);
                }

                if let Some(file_extension) = file_extension {
                    let new_path = path.with_extension(file_extension);
                    replaced = None;
                    path = replaced.get_or_insert(new_path);
                }

                let path = path.to_path(&root);

                if !index.remove(&path) {
                    failure::bail!("did not expect to censor file: {}", path.display());
                }

                let dest = dest_root.join(
                    path.file_name()
                        .ok_or_else(|| failure::format_err!("expected file name"))?,
                );

                tasks.push(Task::Process(path, dest, &f.replace))
            }

            if !index.is_empty() {
                println!(
                    "missing censor configuration for {} file(s) (--list to see them)",
                    index.len()
                );

                for path in index {
                    if list {
                        println!("{}: {}", config_path.display(), path.display());
                    }

                    let dest = dest_root.join(
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .ok_or_else(|| failure::format_err!("expected file name"))?,
                    );

                    tasks.push(Task::ProcessSilent(path, dest));
                }
            }
        }
    }

    tasks
        .into_par_iter()
        .map(|t| t.run())
        .collect::<Result<(), _>>()?;

    Ok(())
}
