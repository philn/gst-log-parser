// Generate input logs with: GST_DEBUG="v4l2allocator:6"
use std::fs::File;
use std::io::Write;
use std::process::exit;

extern crate gst_log_parser;
use gst_log_parser::parse;

extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "v4l2-perf",
    about = "Generate a data file from v4l2 logs"
)]
struct Opt {
    #[structopt(help = "Input file")]
    input: String,
    #[structopt(help = "Output file")]
    output: String,
}

fn generate() -> Result<bool, std::io::Error> {
    let opt = Opt::from_args();
    let input = File::open(opt.input)?;
    let mut output = (File::create(&opt.output))?;

    let parsed = parse(input).filter(|entry| {
        entry.category == "v4l2allocator"
            && entry.function == "gst_v4l2_allocator_dqbuf"
            && entry.message.starts_with("dequeued buffer")
    });

    for entry in parsed {
        let ts = entry.ts.nanoseconds().expect("missing ts");
        let object = entry.object.unwrap();
        let elt_name = object.split(":").next().unwrap();

        write!(output, "{}_{} 1 {}\n", elt_name, "DQBUF", ts)?;
        write!(output, "{}_{} 0 {}\n", elt_name, "DQBUF", ts + 1)?;
    }

    println!("Generated {}", opt.output);
    Ok(true)
}

fn main() {
    if generate().is_err() {
        exit(1);
    }
}
