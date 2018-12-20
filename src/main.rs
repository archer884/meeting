use chrono::{Date, DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::path::PathBuf;
use std::{error, result};
use structopt::StructOpt;

type AppResult<T> = result::Result<T, Box<error::Error>>;

#[derive(Debug, StructOpt)]
#[structopt(name = "meeting", about = "Meeting time logger")]
enum Opt {
    #[structopt(name = "log", about = "Log a meeting")]
    Log {
        /// The hour the meeting started
        ///
        /// Enter this value in military time, e.g. 1:00 PM is "13"
        start: u32,
        /// The length of the meeting in minutes
        length: u32,
    },

    // Format is 2018-12-19
    #[structopt(name = "list", about = "List meetings for a given timeframe")]
    List {
        /// Beginning of date range, e.g. 2018-12-1
        start: NaiveDate,
        /// End of date range, e.g. 2018-12-31
        end: NaiveDate,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Meeting {
    timestamp: DateTime<Utc>,
    length: u32,
}

impl Meeting {
    fn today(start: u32, length: u32) -> Meeting {
        Meeting {
            timestamp: Utc::today().and_hms(start, 0, 0),
            length,
        }
    }

    fn is_within_range(&self, start: Date<Utc>, end: Date<Utc>) -> bool {
        self.timestamp.date() >= start && self.timestamp.date() <= end
    }
}

impl Display for Meeting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {} minutes",
            self.timestamp.format("%F %R"),
            self.length
        )
    }
}

fn main() -> AppResult<()> {
    App::execute(Opt::from_args())
}

struct App;

impl App {
    fn execute(opt: Opt) -> AppResult<()> {
        match opt {
            Opt::Log { start, length } => log(start, length),
            Opt::List { start, end } => list(start, end),
        }
    }
}

fn log(start: u32, length: u32) -> AppResult<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let meeting = Meeting::today(start, length);
    let path = log_path()?;

    let mut log = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut log, &meeting)?;
    log.write_all("\n".as_bytes())?;

    Ok(())
}

fn list(start: NaiveDate, end: NaiveDate) -> AppResult<()> {
    use std::fs;

    fn parse(s: &str) -> Option<Meeting> {
        serde_json::from_str(s).ok()
    }

    let start = to_utc_date(start)?;
    let end = to_utc_date(end)?;
    let content = fs::read_to_string(&log_path()?)?;
    let mut records: Vec<_> = content
        .lines()
        .filter_map(parse)
        .filter(|x| x.is_within_range(start, end))
        .collect();

    records.sort_by_key(|x| x.timestamp);

    let mut minutes = 0;
    for record in &records {
        minutes += record.length;
        println!("{}", record);
    }

    println!("Total hours: {:.1}", f64::from(minutes) / 60.0);

    Ok(())
}

fn log_path() -> AppResult<PathBuf> {
    use directories::UserDirs;

    let path = UserDirs::new()
        .ok_or("Unable to access user directories")?
        .home_dir()
        .join(".meetings");

    Ok(path)
}

fn to_utc_date(date: NaiveDate) -> AppResult<Date<Utc>> {
    use chrono::offset::{LocalResult, TimeZone};
    use std::cmp;

    // Genesis, chapter 3.
    match Utc.from_local_date(&date) {
        LocalResult::None => Err("Invalid date")?,
        LocalResult::Single(date) => Ok(date),
        LocalResult::Ambiguous(left, right) => Ok(cmp::min(left, right)),
    }
}
