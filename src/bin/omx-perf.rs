// Generate input logs with: GST_DEBUG="OMX_PERFORMANCE:8"
use std::fs::File;
use std::io::Write;
use std::process::exit;
use std::collections::HashMap;

extern crate gst_log_parser;
use gst_log_parser::parse;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "omx-perf", about = "Generate a data file from OMX performance logs")]
struct Opt {
    #[structopt(help = "Input file")]
    input: String,
    #[structopt(help = "Output file")]
    output: String,
}

struct Count {
    empty_call: u32,
    empty_done: u32,
    fill_call: u32,
    fill_done: u32,
}

impl Count {
    fn new() -> Count {
        Count {
            fill_call: 0,
            fill_done: 0,
            empty_call: 0,
            empty_done: 0,
        }
    }
}

fn generate() -> Result<bool, std::io::Error> {
    let opt = Opt::from_args();
    let input = File::open(opt.input)?;
    let mut output = (File::create(&opt.output))?;

    let parsed = parse(input).filter(|entry| entry.unwrap().category == "OMX_PERFORMANCE");
    let mut counts: HashMap<String, Count> = HashMap::new();

    for entry in parsed {
        let entry = entry.unwrap();
        let object = entry.object.unwrap();
        // Extract the component name by taking the 4th last chars of the gst object name
        if let Some((i, _)) = object.char_indices().rev().nth(3) {
            let comp_name = &object[i..];
            let ts = entry.ts.nanoseconds().expect("missing ts");
            write!(output, "{}_{} 1 {}\n", comp_name, entry.message, ts)?;
            write!(output, "{}_{} 0 {}\n", comp_name, entry.message, ts + 1)?;

            let count = counts.entry(comp_name.to_string()).or_insert(Count::new());

            match entry.message.as_ref() {
                "EmptyThisBuffer" => count.empty_call += 1,
                "EmptyBufferDone" => count.empty_done += 1,
                "FillThisBuffer" => count.fill_call += 1,
                "FillBufferDone" => count.fill_done += 1,
                _ => (),
            }
        }
    }

    for (comp, count) in &counts {
        println!("{}:", comp);
        println!(
            "\tInput (EmptyBufferDone/EmptyThisBuffer): {}/{}",
            count.empty_done,
            count.empty_call
        );
        println!(
            "\tOutput (FillBufferDone/FillThisBuffer):  {}/{}",
            count.fill_done,
            count.fill_call
        );
    }

    println!("Generated {}", opt.output);
    Ok(true)
}

fn main() {
    if generate().is_err() {
        exit(1);
    }
}
