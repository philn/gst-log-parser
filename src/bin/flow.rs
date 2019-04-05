// Generate input logs with: GST_DEBUG="GST_TRACER:7" GST_TRACERS=stats

use failure::Error;
use gst_log_parser::parse;
use gstreamer::{ClockTime, DebugLevel, Structure};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug, PartialEq, Copy, Clone)]
#[structopt(name = "command")]
enum Command {
    #[structopt(name = "check-decreasing-pts", about = "Check for decreasing PTS")]
    DecreasingPts,
    #[structopt(name = "check-decreasing-dts", about = "Check for decreasing DTS")]
    DecreasingDts,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "flow", about = "Process logs generated by the 'stats' tracer")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug)]
struct Element {
    name: String,
}

impl Element {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Debug)]
struct Pad {
    name: String,
    last_buffer_pts: ClockTime,
    last_buffer_dts: ClockTime,
    element_name: Option<String>,
}

impl Pad {
    fn new(name: &str, element_name: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            last_buffer_pts: ClockTime::none(),
            last_buffer_dts: ClockTime::none(),
            element_name,
        }
    }
}

impl fmt::Display for Pad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.element_name {
            None => write!(f, "{}", self.name),
            Some(e) => write!(f, "{}:{}", e, self.name),
        }
    }
}

#[derive(Debug)]
struct Flow {
    command: Command,
    elements: HashMap<u32, Element>,
    pads: HashMap<u32, Pad>,
}

impl Flow {
    fn new(command: Command) -> Self {
        Self {
            command,
            elements: HashMap::new(),
            pads: HashMap::new(),
        }
    }

    fn parse(&mut self, s: &Structure) {
        match s.get_name() {
            "new-element" => {
                let idx = s.get::<u32>("ix").unwrap();
                self.elements
                    .entry(idx)
                    .or_insert_with(|| Element::new(s.get::<&str>("name").unwrap()));
            }
            "new-pad" => {
                let idx = s.get::<u32>("ix").unwrap();
                let parent_ix = s.get::<u32>("parent-ix").unwrap();
                let element_name = match self.elements.get(&parent_ix) {
                    None => None,
                    Some(e) => Some(e.name.clone()),
                };

                self.pads
                    .entry(idx)
                    .or_insert_with(|| Pad::new(s.get::<&str>("name").unwrap(), element_name));
            }
            "buffer" => {
                self.handle_buffer(s);
            }
            _ => {}
        }
    }

    fn handle_buffer(&mut self, s: &Structure) {
        let pad = self
            .pads
            .get_mut(&s.get::<u32>("pad-ix").unwrap())
            .expect("Unknown pad");
        let element = self
            .elements
            .get(&s.get::<u32>("element-ix").unwrap())
            .expect("Unknown element");

        if pad.element_name.is_none() {
            pad.element_name = Some(element.name.clone());
        }

        if s.get::<bool>("have-buffer-pts").unwrap() {
            let pts = ClockTime::from_nseconds(s.get::<u64>("buffer-pts").unwrap());

            if self.command == Command::DecreasingPts
                && pad.last_buffer_pts.is_some()
                && pts < pad.last_buffer_pts
            {
                println!("Decreasing pts {} {} < {}", pad, pts, pad.last_buffer_pts);
            }
            pad.last_buffer_pts = pts;
        }

        if s.get::<bool>("have-buffer-dts").unwrap() {
            let dts = ClockTime::from_nseconds(s.get::<u64>("buffer-dts").unwrap());

            if self.command == Command::DecreasingPts
                && pad.last_buffer_dts.is_some()
                && dts < pad.last_buffer_dts
            {
                println!("Decreasing dts {} {} < {}", pad, dts, pad.last_buffer_dts);
            }
            pad.last_buffer_dts = dts;
        }
    }
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    let input = File::open(opt.input)?;
    let mut flow = Flow::new(opt.command);

    let parsed = parse(input)
        .filter(|entry| entry.category == "GST_TRACER" && entry.level == DebugLevel::Trace);

    for entry in parsed {
        let s = match entry.message_to_struct() {
            None => continue,
            Some(s) => s,
        };

        flow.parse(&s);
    }

    Ok(())
}
