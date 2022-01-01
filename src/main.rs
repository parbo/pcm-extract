// -*- coding: utf-8 -*-
use clap::{App, Arg};
use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;
use std::io::Read;

fn main() {
    env_logger::init();

    let matches = App::new("PCM Extract")
        .version("0.1")
        .author("Pär Bohrarper <par@bohrarper.se>")
        .about("Extract PCM samples")
        .arg(
            Arg::with_name("INPUT")
                .short("i")
                .long("input")
                .help("Sets the input file to use")
                .required(true)
                .takes_value(true)
        )
        .arg(
            Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .help("Sets the output file to use")
                .required(true)
                .takes_value(true)
        )
        .get_matches();

    let filename = &matches.value_of("INPUT").unwrap();
    error!("opening: {}", filename);
    let mut file = fs::File::open(Path::new(filename)).unwrap();
    let mut input = vec![];
    file.read_to_end(&mut input).unwrap();


    let mut out = vec![];
    let mut ix = 0;
    loop {
	let d = input[ix] as i16;
        out.push(d.overflowing_mul(256).0);
	ix += 2;
	if ix >= input.len() {
	    break;
	}
    }
    let out_filename = &matches.value_of("OUTPUT").unwrap();
    let mut out_file = fs::File::create(Path::new(out_filename)).unwrap();
    let h = wav::Header::new(wav::WAV_FORMAT_PCM, 1, 16000, 16);
    wav::write(h, &wav::BitDepth::Sixteen(out), &mut out_file).unwrap();
}
