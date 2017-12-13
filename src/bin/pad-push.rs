// Generate data with ts from gst_pad_chain_data_unchecked
// Need logs with GST_DEBUG="GST_SCHEDULING:5"
use std::fs::File;
use std::io::Write;
use std::process::exit;
use std::collections::HashMap;

extern crate gst_log_parser;
use gst_log_parser::{parse, parse_time};

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "omx-perf", about = "Generate a data file with timestamp of pushed buffers")]
struct Opt {
    #[structopt(help = "Input file")]
    input: String,
}

fn generate() -> Result<bool, std::io::Error> {
    let opt = Opt::from_args();
    let input = File::open(opt.input)?;
    let mut outputs: HashMap<String, File> = HashMap::new();

    let parsed = parse(input)
        .filter(|entry| entry.category == "GST_SCHEDULING")
        .filter(|entry| entry.function == "gst_pad_chain_data_unchecked")
        .filter(|entry| entry.message.starts_with("calling chainfunction"));

    for entry in parsed {
        let pts = entry.message.split("pts ").nth(1).unwrap().split(",").next().unwrap();
        let pts = parse_time(pts);

        let object = entry.object.unwrap();
        let object_name = object.split(":").next().unwrap();
        //println!("{} {} {}", entry.ts, object_name, pts);

        println!("{} -> {:?}", object_name, outputs.get(object_name));
        let out = outputs.entry(object_name.to_string()).or_insert(File::create(&object_name).unwrap());
        write!(out, "{} {} {}\n", entry.ts, object_name, pts)?;
        write!(out, "coucou\n")?;
    }

    Ok(true)
}

fn main() {
    if generate().is_err() {
        exit(1);
    }
}
