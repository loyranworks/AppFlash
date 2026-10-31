//! Small local-only diagnostics. Retention uses UTC calendar days: today and the
//! previous six days. Only regular `appflash-YYYY-MM-DD.log` files are owned.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use time::{Date, Duration, Month, OffsetDateTime};

const DAILY_LIMIT: u64 = 2 * 1024 * 1024;
const CAP_MARKER: &str = "[diagnostics] daily 2 MiB limit reached; further entries skipped.\n";
const MESSAGE_LIMIT: usize = 16 * 1024;
static LOGGER: OnceLock<Logger> = OnceLock::new();

pub struct Logger {
    directory: PathBuf,
    version: String,
    access: Mutex<()>,
}

pub fn init(directory: PathBuf, version: &str) -> io::Result<()> {
    let logger = Logger::new(directory, version)?;
    let _ = logger.maintenance_at(SystemTime::now());
    LOGGER.set(logger).map_err(|_| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "diagnostics already initialized",
        )
    })
}

/// Recording failures are intentionally ignored and never recursively logged.
pub fn record(category: &str, message: &str) {
    if let Some(logger) = LOGGER.get() {
        let _ = logger.record_at(SystemTime::now(), category, message);
    }
}

pub fn maintenance() {
    if let Some(logger) = LOGGER.get() {
        let _ = logger.maintenance_at(SystemTime::now());
    }
}

pub fn export(destination: &Path) -> Result<(), String> {
    LOGGER
        .get()
        .ok_or_else(|| "Local diagnostics are unavailable (initialization failed).".to_string())?
        .export_at(destination, SystemTime::now())
        .map_err(|error| format!("Could not export local diagnostics: {error}"))
}

impl Logger {
    pub fn new(directory: PathBuf, version: &str) -> io::Result<Self> {
        fs::create_dir_all(&directory)?;
        if !directory.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "diagnostics path is not a directory",
            ));
        }
        Ok(Self {
            directory,
            version: single_line(version, 128),
            access: Mutex::new(()),
        })
    }

    pub fn record_at(&self, now: SystemTime, category: &str, message: &str) -> io::Result<()> {
        let timestamp = utc(now)?;
        let line = format!(
            "{} [{}] {}\n",
            timestamp_text(timestamp),
            single_line(category, 128),
            single_line(message, MESSAGE_LIMIT),
        );
        let _guard = self
            .access
            .lock()
            .map_err(|_| io::Error::other("diagnostics lock unavailable"))?;
        let path = self
            .directory
            .join(format!("appflash-{}.log", date_text(timestamp.date())));
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err(io::Error::other("diagnostics file is not a regular file"));
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
        let length = file.metadata()?.len();
        if length >= DAILY_LIMIT {
            return Ok(());
        }
        // Preserve the cap across restarts without an unbounded in-memory day map.
        if length >= CAP_MARKER.len() as u64 {
            file.seek(SeekFrom::End(-(CAP_MARKER.len() as i64)))?;
            let mut tail = vec![0; CAP_MARKER.len()];
            file.read_exact(&mut tail)?;
            if tail == CAP_MARKER.as_bytes() {
                return Ok(());
            }
        }
        if length + line.len() as u64 + CAP_MARKER.len() as u64 <= DAILY_LIMIT {
            file.write_all(line.as_bytes())?;
        } else if length + CAP_MARKER.len() as u64 <= DAILY_LIMIT {
            file.write_all(CAP_MARKER.as_bytes())?;
        }
        Ok(())
    }

    pub fn maintenance_at(&self, now: SystemTime) -> io::Result<()> {
        let earliest = utc(now)?.date() - Duration::days(6);
        let _guard = self
            .access
            .lock()
            .map_err(|_| io::Error::other("diagnostics lock unavailable"))?;
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            if owned_date(&entry.path()).is_some_and(|date| date < earliest) {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }

    pub fn export_at(&self, destination: &Path, now: SystemTime) -> io::Result<()> {
        let timestamp = utc(now)?;
        let today = timestamp.date();
        let earliest = today - Duration::days(6);
        let mut contents = format!(
            "AppFlash local diagnostics\nVersion: {}\nSystem: {} / {}\nExported: {} (UTC)\nRetention: {} through {} (seven UTC calendar days)\n\n",
            self.version, std::env::consts::OS, std::env::consts::ARCH,
            timestamp_text(timestamp), date_text(earliest), date_text(today),
        );
        {
            let _guard = self
                .access
                .lock()
                .map_err(|_| io::Error::other("diagnostics lock unavailable"))?;
            let mut files = Vec::new();
            for entry in fs::read_dir(&self.directory)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }
                if let Some(date) = owned_date(&entry.path()) {
                    if date >= earliest && date <= today {
                        files.push((date, entry.path()));
                    }
                }
            }
            files.sort_by_key(|(date, _)| *date);
            let destination_absolute = if destination.is_absolute() {
                destination.to_path_buf()
            } else {
                std::env::current_dir()?.join(destination)
            };
            for (date, path) in files {
                if fs::canonicalize(&path)?
                    == fs::canonicalize(&destination_absolute)
                        .unwrap_or(destination_absolute.clone())
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "export destination is an active diagnostics file",
                    ));
                }
                // All owned files are capped; limit reads as well if a file was replaced externally.
                let mut text = String::new();
                File::open(path)?
                    .take(DAILY_LIMIT)
                    .read_to_string(&mut text)?;
                contents.push_str(&format!("--- {} (UTC) ---\n", date_text(date)));
                contents.push_str(&text);
                if !text.ends_with('\n') {
                    contents.push('\n');
                }
                contents.push('\n');
            }
        }
        // Slow destination IO happens after releasing the append mutex.
        let mut file = File::create(destination)?;
        file.write_all(contents.as_bytes())?;
        file.flush()
    }
}

fn utc(now: SystemTime) -> io::Result<OffsetDateTime> {
    let nanoseconds = match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as i128,
        Err(error) => -(error.duration().as_nanos() as i128),
    };
    OffsetDateTime::from_unix_timestamp_nanos(nanoseconds)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

fn date_text(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

fn timestamp_text(timestamp: OffsetDateTime) -> String {
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        date_text(timestamp.date()),
        timestamp.hour(),
        timestamp.minute(),
        timestamp.second()
    )
}

fn owned_date(path: &Path) -> Option<Date> {
    let name = path.file_name()?.to_str()?;
    let date = name.strip_prefix("appflash-")?.strip_suffix(".log")?;
    if date.len() != 10 || !date.is_ascii() {
        return None;
    }
    let mut fields = date.split('-');
    let year = fields.next()?.parse().ok()?;
    let month = Month::try_from(fields.next()?.parse::<u8>().ok()?).ok()?;
    let day = fields.next()?.parse().ok()?;
    let parsed = Date::from_calendar_date(year, month, day).ok()?;
    (fields.next().is_none() && date_text(parsed) == date).then_some(parsed)
}

fn single_line(text: &str, limit: usize) -> String {
    let mut result = String::with_capacity(text.len().min(limit));
    for character in text.chars() {
        let character = if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') {
            ' '
        } else {
            character
        };
        if result.len() + character.len_utf8() > limit.saturating_sub('…'.len_utf8()) {
            result.push('…');
            break;
        }
        result.push(character);
    }
    result
}
