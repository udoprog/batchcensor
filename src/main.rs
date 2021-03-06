use batchcensor::{generator, utils, Config, Generator, Pos, Replace, Transcript};
use failure::ResultExt;
use relative_path::{RelativePath, RelativePathBuf};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    fs::File,
    io,
    path::{Path, PathBuf},
};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

struct Missing<'a>(&'a Path, &'a Path, &'a RelativePath);

/// A single task that can be executed.
pub enum Task<'a> {
    /// Copy a single file.
    Copy(PathBuf, PathBuf),
    /// Regular processing with replacements.
    Process(PathBuf, PathBuf, Vec<&'a Replace>),
    // Silent processing.
    Silence(PathBuf, PathBuf),
}

impl<'a> Task<'a> {
    fn run(&self, generator: &dyn Generator) -> Result<(), failure::Error> {
        match *self {
            Task::Copy(ref path, ref dest) => {
                process_copy(path, dest)?;
            }
            Task::Process(ref path, ref dest, ref replace) => {
                process_single(&path, &dest, replace, generator)?;
            }
            Task::Silence(ref path, ref dest) => {
                process_silent(&path, &dest)?;
            }
        }

        Ok(())
    }
}

impl<'a> fmt::Display for Task<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Task::Copy(ref path, ref dest) => {
                write!(fmt, "copy {} -> {}", path.display(), dest.display())?;
            }
            Task::Process(ref path, ref dest, ..) => {
                write!(fmt, "process {} -> {}", path.display(), dest.display())?;
            }
            Task::Silence(ref path, ref dest) => {
                write!(fmt, "silence {} -> {}", path.display(), dest.display())?;
            }
        }

        Ok(())
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
                .value_name("file")
                .help("Configuration file to use.")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("config-dir")
                .short("d")
                .long("config-dir")
                .value_name("dir")
                .help("Configuration directory to use.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("root")
                .short("r")
                .long("root")
                .value_name("dir")
                .help("Root of project to process.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("dir")
                .help("Where to build output.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("list")
                .long("list")
                .help("List files which will be muted since they don't have a configuration."),
        )
        .arg(
            clap::Arg::with_name("stats")
                .long("stats")
                .help("Show statistics about all configurations loaded."),
        )
        .arg(
            clap::Arg::with_name("init")
                .long("init")
                .help("Initialize an existing configuration, complete with missing files.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("oiv-manifest")
                .long("oiv-manifest")
                .value_name("file")
                .help("Where to write the GTAV .oiv manifest.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("tone")
                .long("tone")
                .help("Replace censored sections with a 1000Hz tone instead of blank audio."),
        )
}

/// Copy a single file.
fn process_copy(path: &Path, dest: &Path) -> Result<(), failure::Error> {
    let dest_parent = dest
        .parent()
        .ok_or_else(|| failure::format_err!("expected destination to have parent dir"))?;

    if !dest_parent.is_dir() {
        std::fs::create_dir_all(dest_parent)?;
    }

    std::fs::copy(path, dest)?;
    Ok(())
}

/// Process a single file and apply all the specified replacements.
fn process_single(
    path: &Path,
    dest_path: &Path,
    replaces: &[&Replace],
    generator: &dyn Generator,
) -> Result<(), failure::Error> {
    let dest_parent = dest_path
        .parent()
        .ok_or_else(|| failure::format_err!("expected destination to have parent dir"))?;

    if !dest_parent.is_dir() {
        std::fs::create_dir_all(dest_parent)?;
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
        let start = pos(range.start.as_ref(), s, duration, 0) as usize;
        let end = pos(range.end.as_ref(), s, duration, duration) as usize;

        if start == end {
            continue;
        }

        let generated = generator.generate(start..end, s.sample_rate);

        if start >= end {
            failure::bail!("{}: {} (start) is not before {} (end)", replace, start, end);
        }

        if start > data.len() || end > data.len() {
            failure::bail!(
                "{}: {}-{} out of range 0-{}",
                replace,
                start,
                end,
                data.len()
            );
        }

        (&mut data[start..end]).copy_from_slice(&generated);
    }

    let d = File::create(&dest_path)?;
    let mut w = hound::WavWriter::new(d, s)?;

    let mut writer = w.get_i16_writer(data.len() as u32);

    for d in data {
        writer.write_sample(d);
    }

    writer.flush()?;
    return Ok(());

    fn pos(pos: Option<&Pos>, s: hound::WavSpec, duration: u32, default: u32) -> u32 {
        match pos.as_ref() {
            Some(pos) => {
                let pos = pos
                    .as_samples(s.sample_rate)
                    .expect("samples overflow with sample rate")
                    .checked_mul(s.channels as u32)
                    .expect("overflow");

                u32::min(pos, duration)
            }
            None => default,
        }
    }
}

/// Replace the given file with silence.
fn process_silent(path: &Path, dest_path: &Path) -> Result<(), failure::Error> {
    if dest_path.is_file() {
        // Ignore files that already exist.
        return Ok(());
    }

    let dest_parent = dest_path
        .parent()
        .ok_or_else(|| failure::format_err!("expected destination to have parent dir"))?;

    if !dest_parent.is_dir() {
        std::fs::create_dir_all(dest_parent)?;
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

/// Write out the .oiv manifest for GTA V.
fn write_oiv_manifest(
    modified: &BTreeSet<RelativePathBuf>,
    output: Option<&Path>,
) -> Result<(), failure::Error> {
    use std::{collections::btree_map::Entry, io::Write};

    let mut archives = BTreeMap::new();

    for m in modified {
        let mut c = m.components();
        let rpf = c.next().expect("expected root").as_str();

        let archive = match archives.entry(rpf.clone()) {
            Entry::Vacant(e) => e.insert(Archive {
                path: format!("x64/audio/sfx/{}.rpf", rpf),
                create_if_not_exists: "True",
                ty: String::from("RPF7"),
                add: Vec::new(),
            }),
            Entry::Occupied(e) => e.into_mut(),
        };

        let audio_file = c.next().expect("expected audio file").as_str();

        archive.add.push(Add {
            source: format!("{}.awc", m.display()),
            value: format!("{}.awc", audio_file),
        });
    }

    let mut content = Content::default();
    content.archives.extend(archives.into_iter().map(|v| v.1));

    match output {
        Some(output) => {
            let mut f = File::create(output)?;
            write!(f, "{}", content)?;
        }
        None => {
            println!("{}", content);
        }
    }

    return Ok(());

    #[derive(Debug)]
    struct Add {
        source: String,
        value: String,
    }

    impl Add {
        pub fn to_xml(&self, fmt: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
            let prefix = std::iter::repeat(' ').take(depth).collect::<String>();

            writeln!(
                fmt,
                "{}<add source=\"{}\">{}</add>",
                prefix, self.source, self.value
            )?;

            Ok(())
        }
    }

    #[derive(Debug)]
    struct Archive {
        path: String,
        create_if_not_exists: &'static str,
        ty: String,
        add: Vec<Add>,
    }

    impl Archive {
        pub fn to_xml(&self, fmt: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
            let prefix = std::iter::repeat(' ').take(depth).collect::<String>();

            writeln!(
                fmt,
                "{}<archive path=\"{}\" createIfNotExist=\"{}\" type=\"{}\">",
                prefix, self.path, self.create_if_not_exists, self.ty
            )?;

            for a in &self.add {
                a.to_xml(fmt, depth + 2)?;
            }

            writeln!(fmt, "{}</archive>", prefix)?;
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct Content {
        archives: Vec<Archive>,
    }

    impl Content {
        pub fn to_xml(&self, fmt: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
            let prefix = std::iter::repeat(' ').take(depth).collect::<String>();

            writeln!(fmt, "{}<content>", prefix)?;

            for a in &self.archives {
                a.to_xml(fmt, depth + 2)?;
            }

            writeln!(fmt, "{}</content>", prefix)?;
            Ok(())
        }
    }

    impl fmt::Display for Content {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.to_xml(fmt, 0)
        }
    }
}

/// Initialize missing files into the current set of configurations.
fn do_init<'a>(
    out: &mut impl io::Write,
    missing: BTreeMap<PathBuf, Missing<'a>>,
    mut configs: Vec<(&'a Path, &'a Path, Config)>,
) -> Result<(), failure::Error> {
    for m in missing {
        for (root, config_path, config) in &mut configs {
            if *config_path != (m.1).0 {
                continue;
            }

            let (path, Missing(_, _, dir_path)) = m;

            let path = path.strip_prefix(&root)?;

            let mut c = path.components();
            for _ in (&mut c).take(dir_path.components().count()) {}
            let path = RelativePath::from_path(c.as_path())?;

            let transcript = Transcript::parse("[missing]")?;
            config.insert_file(dir_path, path.to_owned(), transcript)?;
            break;
        }
    }

    // optimize all configurations.
    for (_, _, config) in &mut configs {
        config.optimize()?;
    }

    for (_, _, config) in &configs {
        serde_yaml::to_writer(&mut *out, &config)?;
    }

    return Ok(());
}

fn main() -> Result<(), failure::Error> {
    use rayon::prelude::*;

    let m = opts().get_matches();
    let list = m.is_present("list");
    let stats = m.is_present("stats");
    let tone = m.is_present("tone");
    let output = m.value_of("output").map(PathBuf::from);
    let init = m.value_of("init");

    let mut counts = BTreeMap::<String, u64>::new();

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
                _ => {}
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

            Ok((root, path.as_path(), config))
        })
        .collect::<Result<Vec<_>, failure::Error>>()?;

    let mut tasks = Vec::new();

    // keep track if we are processing any files, which will determine what goes into the manifest.
    let mut modified = BTreeSet::new();

    let mut missing = BTreeMap::<PathBuf, Missing>::new();
    let mut silenced = BTreeMap::<PathBuf, Missing>::new();
    let mut roots = HashMap::new();
    let mut dirs = HashMap::<PathBuf, Vec<_>>::new();

    // Go through all configurations and construct root directories.
    for (root, config_path, config) in &configs {
        let output = output
            .as_ref()
            .cloned()
            .unwrap_or_else(|| root.join("output"));

        for dir in &config.dirs {
            let root = dir.path.to_path(&root);

            if !root.is_dir() {
                failure::bail!("no such directory: {}", root.display());
            }

            dirs.entry(root.clone()).or_default().push(dir);

            let mut dest_root = output.to_owned();

            for c in dir.path.components() {
                dest_root.push(c.as_str());
            }

            roots.insert(root, (dest_root, *config_path, config, &dir.path));
        }
    }

    for (root, (dest_root, config_path, config, dir_path)) in &roots {
        if !root.is_dir() {
            failure::bail!("no such directory: {}", root.display());
        }

        // copy corresponding .oac file if present.
        {
            let oac = root.with_extension("oac");

            if oac.is_file() {
                tasks.push(Task::Copy(oac, dest_root.with_extension("oac")));
            }
        }

        for result in ignore::Walk::new(&root) {
            let result = result?;
            let path = result.path().to_owned();

            if !path.is_file() {
                continue;
            }

            match path.extension().and_then(|s| s.to_str()) {
                Some("wav") => {}
                _ => {
                    let dest = dest_root.join(path.strip_prefix(&root)?);
                    // NB: straight up copy other files.
                    tasks.push(Task::Copy(path, dest));
                    continue;
                }
            }

            // Keep track of all files to produce a list of files missing configuration in the end.
            missing.insert(path, Missing(config_path, dest_root, *dir_path));
        }

        // Process all dirs.
        for dir in dirs.get(root).into_iter().flat_map(|r| r) {
            for (i, (path, mut replace, transcript)) in dir.files.iter().enumerate() {
                let file_extension = dir
                    .file_extension
                    .as_ref()
                    .or(config.file_extension.as_ref());

                // temp storage for modified path so that we can continue dealing with references.
                let mut path = Cow::Borrowed(path);

                // replace a `$$` in any component present with the current enumeration.
                path = utils::path_enumeration(i, path);
                path = utils::path_file_prefix(dir.prefix.as_ref().map(|s| s.as_str()), path);
                path = utils::path_file_suffix(dir.suffix.as_ref().map(|s| s.as_str()), path);

                if let Some(file_extension) = file_extension {
                    path = Cow::Owned(path.with_extension(file_extension));
                }

                let path = path.to_path(&root);

                let dest = dest_root.join(
                    path.file_name()
                        .ok_or_else(|| failure::format_err!("expected file name"))?,
                );

                let indexed = match missing.remove(&path) {
                    Some(indexed) => indexed,
                    None => {
                        failure::bail!("did not expect to censor file: {}", path.display());
                    }
                };

                if let Some(transcript) = transcript {
                    // file silenced because it has marked words which do not have a range.
                    if !transcript.missing.is_empty() {
                        silenced.insert(path.clone(), indexed);
                        tasks.push(Task::Silence(path, dest));
                        continue;
                    }

                    replace.extend(transcript.replace.iter());
                }

                // audio file already clean.
                if replace.is_empty() {
                    tasks.push(Task::Copy(path, dest));
                    continue;
                }

                if stats {
                    for r in replace.iter().cloned() {
                        *counts.entry(r.word.to_lowercase()).or_default() += 1;
                    }
                }

                modified.insert(dir.path.to_owned());
                tasks.push(Task::Process(path, dest, replace));
            }
        }
    }

    if init.is_some() {
        if missing.is_empty() {
            println!("nothing to initialize: there are no missing files!");
            return Ok(());
        }

        match init {
            None | Some("-") => {
                let out = io::stdout();
                return do_init(&mut out.lock(), missing, configs.clone());
            }
            Some(other) => {
                let other = Path::new(other);

                let mut f = File::create(other).with_context(|_| {
                    failure::format_err!(
                        "failed to open init file for writing: {}",
                        other.display()
                    )
                })?;

                return do_init(&mut f, missing, configs.clone());
            }
        }
    }

    if !missing.is_empty() || !silenced.is_empty() {
        if !list {
            if !missing.is_empty() {
                eprintln!(
                    "Missing censor configuration for {} file(s) (--list to see them)",
                    missing.len()
                );
            }

            if !silenced.is_empty() {
                eprintln!(
                    "Silenced censor configuration for {} file(s) (--list to see them)",
                    silenced.len()
                );
            }
        } else {
            for (path, Missing(config_path, ..)) in &missing {
                eprintln!(
                    "{}: missing config for: {}",
                    config_path.display(),
                    path.display()
                );
            }

            for (path, Missing(config_path, ..)) in &silenced {
                eprintln!(
                    "{}: silenced config for: {}",
                    config_path.display(),
                    path.display()
                );
            }
        }

        for (path, Missing(_, dest_root, file)) in missing.into_iter().chain(silenced.into_iter()) {
            let dest = dest_root.join(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| failure::format_err!("expected file name"))?,
            );

            modified.insert(file.to_owned());
            tasks.push(Task::Silence(path, dest));
        }
    }

    if stats {
        println!("# Statistics (--stats)");

        for (word, count) in counts {
            println!("{} - {}", word, count);
        }
    } else {
        let pb = indicatif::ProgressBar::new(tasks.len() as u64);

        let generator = if tone {
            Box::new(generator::Tone::new()) as Box<dyn Generator>
        } else {
            Box::new(generator::Silence::new()) as Box<dyn Generator>
        };

        tasks
            .into_par_iter()
            .map(|t| {
                let r = t
                    .run(&*generator)
                    .with_context(|_| failure::format_err!("failed to run: {}", t));
                pb.inc(1);
                r
            })
            .collect::<Result<(), _>>()?;

        pb.finish();
    }

    if let Some(oiv_manifest) = m.value_of("oiv-manifest") {
        let out = match oiv_manifest {
            "-" => None,
            other => Some(Path::new(other)),
        };

        write_oiv_manifest(&modified, out)?;
    }

    Ok(())
}
