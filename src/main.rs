use chrono::{Date, DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::fs;
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

    /// List meetings within a given range
    ///
    /// Provide a start and end date to list all meetings occurring within that date range.
    /// A single date will list only meetings occurring on that day. Providing no dates at all
    /// will list only meetings for the current day.
    #[structopt(name = "list", about = "List meetings for a given timeframe")]
    List {
        /// e.g. 2018-12-31
        start: Option<NaiveDate>,
        /// e.g. 2018-12-31
        end: Option<NaiveDate>,
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
            timestamp: Local::today().and_hms(start, 0, 0).with_timezone(&Utc),
            length,
        }
    }

    fn is_within_range(&self, start: Date<Local>, end: Date<Local>) -> bool {
        let compare = self.timestamp.with_timezone(&Local).date();
        compare >= start && compare <= end
    }
}

impl Display for Meeting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let local = self.timestamp.with_timezone(&Local);
        write!(
            f,
            "{}: {} minutes",
            local.format("%F %R"),
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

fn list(start: Option<NaiveDate>, end: Option<NaiveDate>) -> AppResult<()> {
    match (start, end) {
        (Some(start), Some(end)) => list_range(start, end),
        (Some(start), None) => list_date(start),
        (None, None) => list_today(),

        _ => unreachable!(
            "Structopt should not fill the second positional argument before the first."
        ),
    }
}

fn list_range(start: NaiveDate, end: NaiveDate) -> AppResult<()> {
    let start = to_local_date(start)?;
    let end = to_local_date(end)?;
    let records = load_records(|x| x.is_within_range(start, end))?;
    print_records(records);
    Ok(())
}

fn list_date(date: NaiveDate) -> AppResult<()> {
    let date = to_local_date(date)?;
    let records = load_records(|x| x.is_within_range(date, date))?;
    print_records(records);
    Ok(())
}

fn list_today() -> AppResult<()> {
    let date = Local::today();
    let records = load_records(|x| x.is_within_range(date, date))?;
    print_records(records);
    Ok(())
}

fn load_records(f: impl FnMut(&Meeting) -> bool) -> AppResult<Vec<Meeting>> {
    let content = fs::read_to_string(&log_path()?)?;
    let mut records: Vec<_> = content
        .lines()
        .filter_map(parse_meeting)
        .filter(f)
        .collect();

    records.sort_by_key(|x| x.timestamp);
    Ok(records)
}

fn print_records(records: impl IntoIterator<Item = Meeting>) {
    let mut minutes = 0;
    for record in records {
        minutes += record.length;
        println!("{}", record);
    }

    println!("Total hours: {:.1}", f64::from(minutes) / 60.0);
}

fn parse_meeting(s: &str) -> Option<Meeting> {
    serde_json::from_str(s).ok()
}

fn log_path() -> AppResult<PathBuf> {
    use directories::UserDirs;

    let path = UserDirs::new()
        .ok_or("Unable to access user directories")?
        .home_dir()
        .join(".meetings");

    Ok(path)
}

fn to_local_date(date: NaiveDate) -> AppResult<Date<Local>> {
    use chrono::offset::{LocalResult, TimeZone};
    use std::cmp;

    // Genesis, chapter 3.
    match Local.from_local_date(&date) {
        LocalResult::None => Err("Invalid date")?,
        LocalResult::Single(date) => Ok(date),
        LocalResult::Ambiguous(left, right) => Ok(cmp::min(left, right)),
    }
}
