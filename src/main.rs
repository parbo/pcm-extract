// -*- coding: utf-8 -*-

use anyhow::{self, Context};
use clap::{App, Arg};
use easy_repl::{command, repl::LoopStatus, CommandStatus, Repl, validator};
use log::{debug, error, info, warn};
use std::fs;
use std::io::Read;
use std::cell::RefCell;
use std::path::Path;
use textplots::{Chart, Plot, Shape};

#[derive(Copy, Clone, Debug)]
struct Opts {
    px: f32,
    py: f32,
    step: usize,
    offset: u8,
    flip: u8,
    mirror: u8,
}

impl Default for Opts {
    fn default() -> Opts {
	Opts {
	    px: 0.0,
	    py: 128.0,
	    step: 2,
	    offset: 0,
	    flip: 0,
	    mirror: 0,
	}
    }
}

fn main() -> anyhow::Result<()> {
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
                .takes_value(true),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .help("Sets the output file to use")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let filename = &matches.value_of("INPUT").unwrap();
    error!("opening: {}", filename);
    let mut file = fs::File::open(Path::new(filename)).unwrap();
    let mut input = vec![];
    file.read_to_end(&mut input).unwrap();

    let opts = RefCell::new(Opts::default());
    let mut out = vec![];

    let mut repl = Repl::builder()
        .add("flip", easy_repl::Command {
            description: "Set flip".into(),
            args_info: vec![],
            handler: Box::new(|args| {
                let validator = validator!(u8);
                validator(args)?;
                opts.borrow_mut().flip = args[0].parse::<u8>()?;
                Ok(CommandStatus::Done)
            })
        })
        .add("mirror", easy_repl::Command {
            description: "Set mirror".into(),
            args_info: vec![],
            handler: Box::new(|args| {
                let validator = validator!(u8);
                validator(args)?;
                opts.borrow_mut().mirror = args[0].parse::<u8>()?;
                Ok(CommandStatus::Done)
            })
        })
        .add("offset", easy_repl::Command {
            description: "Set offset".into(),
            args_info: vec![],
            handler: Box::new(|args| {
                let validator = validator!(u8);
                validator(args)?;
                opts.borrow_mut().offset = args[0].parse::<u8>()?;
                Ok(CommandStatus::Done)
            }),
        })
        .add("step", easy_repl::Command {
            description: "Set step".into(),
            args_info: vec![],
            handler: Box::new(|args| {
                let validator = validator!(usize);
                validator(args)?;
                opts.borrow_mut().step = args[0].parse::<usize>()?;
                Ok(CommandStatus::Done)
            }),
        })
        .add("range", easy_repl::Command {
            description: "Set plot range".into(),
            args_info: vec![],
            handler: Box::new(|args| {
                let validator = validator!(f32, f32);
                validator(args)?;
		opts.borrow_mut().px = args[0].parse::<f32>()?;
                opts.borrow_mut().py = args[1].parse::<f32>()?;
                Ok(CommandStatus::Done)
            }),
        })
        .build()
        .expect("Failed to create repl");

    loop {
	let mut ix = 0;
	out.clear();
	let opt = opts.borrow().clone();
        loop {
	    let mut d8 = input[ix];
	    let f = opt.flip;
	    let m = opt.mirror;
	    if d8 > m {
                d8 = m + d8.overflowing_sub(m).0;
	    }
	    if d8 < f {
                d8 = f.overflowing_sub(d8).0;
	    }
            let mut d = (d8 as i8) as i16;
            d = d.overflowing_sub(opt.offset as i16).0;
            out.push(d.overflowing_mul(256).0);
            ix += opt.step;
            if ix >= input.len() {
                break;
            }
        }

        let mut plt = vec![];
        for (i, x) in out.iter().enumerate() {
            plt.push((i as f32, *x as f32));
        }
        Chart::new(300, 60, opt.px, opt.py)
            .lineplot(&Shape::Steps(&plt))
            .display();
        let mut plt2 = vec![];
        for (i, x) in input.iter().step_by(opt.step).enumerate() {
            plt2.push((i as f32, *x as f32));
        }
        Chart::new(300, 60, opt.px, opt.py)
            .lineplot(&Shape::Steps(&plt2))
            .display();
        if let Ok(LoopStatus::Continue) = repl.next() {
        } else {
            break;
        }
    }

    let out_filename = &matches.value_of("OUTPUT").unwrap();
    let mut out_file = fs::File::create(Path::new(out_filename)).unwrap();
    let h = wav::Header::new(wav::WAV_FORMAT_PCM, 1, 16000, 16);
    let out_copy = out.clone();
    wav::write(h, &wav::BitDepth::Sixteen(out_copy), &mut out_file).unwrap();

    Ok(())
}
