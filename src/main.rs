// -*- coding: utf-8 -*-

use anyhow::{self};
use clap::{App, Arg};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use easy_repl::{repl::LoopStatus, validator, CommandStatus, Repl};
use std::cell::RefCell;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Barrier};
use textplots::{Chart, Plot, Shape};

#[derive(parse_display::Display, parse_display::FromStr, Debug, Copy, Clone)]
#[display(style = "kebab-case")]
enum Representation {
    SignedMagnitude,
    OnesComplement,
    TwosComplement,
    ExcessK,
    Custom,
}

#[derive(parse_display::Display, parse_display::FromStr, Debug, Copy, Clone)]
#[display(style = "lowercase")]
enum Compression {
    DPCM0,
    DPCM1,
    DPCM2,
    DPCM3,
}

#[derive(Copy, Clone, Debug)]
struct Opts {
    from: usize,
    to: usize,
    step: usize,
    offset: u8,
    flip: u8,
    mirror: u8,
    sign: u8,
    representation: Representation,
    compression: Compression,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            from: 0,
            to: 128,
            step: 2,
            offset: 0,
            flip: 0,
            mirror: 0,
            sign: 1,
            representation: Representation::TwosComplement,
            compression: Compression::DPCM0,
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let config = device.default_output_config().unwrap();

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
    let mut file = fs::File::open(Path::new(filename)).unwrap();
    let mut input = vec![];
    file.read_to_end(&mut input).unwrap();

    let opts = RefCell::new(Opts::default());
    let play = RefCell::new(false);
    let mut out = vec![];

    let mut repl = Repl::builder()
        .add(
            "flip",
            easy_repl::Command {
                description: "Set flip".into(),
                args_info: vec![],
                handler: Box::new(|args| {
                    let validator = validator!(u8);
                    validator(args)?;
                    opts.borrow_mut().flip = args[0].parse::<u8>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "mirror",
            easy_repl::Command {
                description: "Set mirror".into(),
                args_info: vec![],
                handler: Box::new(|args| {
                    let validator = validator!(u8);
                    validator(args)?;
                    opts.borrow_mut().mirror = args[0].parse::<u8>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "sign",
            easy_repl::Command {
                description: "Set sign bit (0=LSB, 1=MSB)".into(),
                args_info: vec![],
                handler: Box::new(|args| {
                    let validator = validator!(u8);
                    validator(args)?;
                    opts.borrow_mut().sign = args[0].parse::<u8>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "offset",
            easy_repl::Command {
                description: "Set offset".into(),
                args_info: vec![],
                handler: Box::new(|args| {
                    let validator = validator!(u8);
                    validator(args)?;
                    opts.borrow_mut().offset = args[0].parse::<u8>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "representation",
            easy_repl::Command {
                description: "Set representation".into(),
                args_info: vec![
                    Representation::SignedMagnitude.to_string(),
                    Representation::OnesComplement.to_string(),
                    Representation::TwosComplement.to_string(),
                    Representation::Custom.to_string(),
                ],
                handler: Box::new(|args| {
                    let validator = validator!(Representation);
                    validator(args)?;
                    opts.borrow_mut().representation = args[0].parse::<Representation>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "compression",
            easy_repl::Command {
                description: "Set compression".into(),
                args_info: vec![
                    Compression::DPCM0.to_string(),
                    Compression::DPCM1.to_string(),
                    Compression::DPCM2.to_string(),
                    Compression::DPCM3.to_string(),
                ],
                handler: Box::new(|args| {
                    let validator = validator!(Compression);
                    validator(args)?;
                    opts.borrow_mut().compression = args[0].parse::<Compression>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "step",
            easy_repl::Command {
                description: "Set step".into(),
                args_info: vec![],
                handler: Box::new(|args| {
                    let validator = validator!(usize);
                    validator(args)?;
                    opts.borrow_mut().step = args[0].parse::<usize>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "range",
            easy_repl::Command {
                description: "Set plot range".into(),
                args_info: vec![],
                handler: Box::new(|args| {
                    let validator = validator!(usize, usize);
                    validator(args)?;
                    opts.borrow_mut().from = args[0].parse::<usize>()?;
                    opts.borrow_mut().to = args[1].parse::<usize>()?;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "+",
            easy_repl::Command {
                description: "Zoom in".into(),
                args_info: vec![],
                handler: Box::new(|_args| {
		    let from = opts.borrow().from;
		    let to = opts.borrow().to;
		    let amount = (to - from) / 4;
                    opts.borrow_mut().from = from + amount;
                    opts.borrow_mut().to = to - amount;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "-",
            easy_repl::Command {
                description: "Zoom out".into(),
                args_info: vec![],
                handler: Box::new(|_args| {
		    let from = opts.borrow().from;
		    let to = opts.borrow().to;
		    let amount = (to - from) / 2;
		    opts.borrow_mut().from = from - amount.min(from);
		    opts.borrow_mut().to = to + amount;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "<",
            easy_repl::Command {
                description: "Move left".into(),
                args_info: vec![],
                handler: Box::new(|_args| {
		    let from = opts.borrow().from;
		    let to = opts.borrow().to;
		    let w = to - from;
		    let amount = w / 2;
		    opts.borrow_mut().from = from - amount.min(from);
		    opts.borrow_mut().to = from - amount.min(from) + w;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            ">",
            easy_repl::Command {
                description: "Move right".into(),
                args_info: vec![],
                handler: Box::new(|_args| {
		    let from = opts.borrow().from;
		    let to = opts.borrow().to;
		    let w = to - from;
		    let amount = w / 2;
		    opts.borrow_mut().from = from + amount;
		    opts.borrow_mut().to = from + amount + w;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .add(
            "play",
            easy_repl::Command {
                description: "Play range".into(),
                args_info: vec![],
                handler: Box::new(|_args| {
                    *play.borrow_mut() = true;
                    Ok(CommandStatus::Done)
                }),
            },
        )
        .build()
        .expect("Failed to create repl");

    loop {
        let mut ix = 0;
        out.clear();
        let opt = opts.borrow().clone();
        loop {
            let mut d8 = input[ix];
            let d = match opt.representation {
                Representation::Custom => {
                    let f = opt.flip;
                    let m = opt.mirror;
                    if d8 > m {
                        d8 = m + d8.overflowing_sub(m).0;
                    }
                    if d8 < f {
                        d8 = f.overflowing_sub(d8).0;
                    }
                    let d = (d8 as i8) as i16;
                    d.overflowing_sub(opt.offset as i16).0
                }
                Representation::OnesComplement => {
                    if d8 < 128 {
                        d8 as i16
                    } else {
                        -(!d8 as i16)
                    }
                }
                Representation::TwosComplement => (d8 as i8) as i16,
                Representation::SignedMagnitude => {
                    if opt.sign == 0 {
                        let sign = d8 & 0x1;
                        if sign == 0 {
                            ((d8 & 0xFE) >> 1) as i16
                        } else {
                            -(((d8 & 0xFE) >> 1) as i16)
                        }
                    } else {
                        let sign = d8 >> 7;
                        if sign == 0 {
                            (d8 & 0x7F) as i16
                        } else {
                            -((d8 & 0x7F) as i16)
                        }
                    }
                }
                Representation::ExcessK => (d8 as i16).overflowing_sub(opt.offset as i16).0,
            };
            match opt.compression {
                Compression::DPCM0 => out.push(d.overflowing_mul(256).0),
                Compression::DPCM1 => {
                    let err = d;
                    let n1 = if out.len() > 0 { out[out.len() - 1] } else { 0 };
                    out.push(n1 + err);
                }
                Compression::DPCM2 => {
                    let err = d;
                    let n1 = if out.len() > 0 { out[out.len() - 1] } else { 0 };
                    let n2 = if out.len() > 1 { out[out.len() - 2] } else { 0 };
                    out.push((2 * n1 - n2) + err);
                }
                Compression::DPCM3 => {
                    let err = d;
                    let n1 = if out.len() > 0 { out[out.len() - 1] } else { 0 };
                    let n2 = if out.len() > 1 { out[out.len() - 2] } else { 0 };
                    let n3 = if out.len() > 2 { out[out.len() - 3] } else { 0 };
                    out.push(3 * n1 - 3 * n2 + n3 + err);
                }
            }
            if ix < 10 {
                println!("d: {}, out: {}", d, out.last().unwrap());
            }
            ix += opt.step;
            if ix >= input.len() {
                break;
            }
        }

        let mut plt = vec![];
        for (i, x) in out.iter().enumerate() {
            plt.push((i as f32, *x as f32));
        }
        Chart::new(300, 60, opt.from as f32, opt.to as f32)
            .lineplot(&Shape::Steps(&plt))
            .display();
        let mut plt2 = vec![];
        for (i, x) in input.iter().step_by(opt.step).enumerate() {
            plt2.push((i as f32, *x as f32));
        }
        Chart::new(300, 60, opt.from as f32, opt.to as f32)
            .lineplot(&Shape::Steps(&plt2))
            .display();

        if *play.borrow() {
            *play.borrow_mut() = false;

            let barrier = Arc::new(Barrier::new(2));

            let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

            let out_copy = out.clone();
            let from = opt.from.min(out_copy.len());
            let to = opt.to.min(out_copy.len());
            let mut frames = 0;

            let c = Arc::clone(&barrier);
            let mut done = false;
            let sc: cpal::StreamConfig = config.clone().into();
            let stream = device.build_output_stream(
                &sc,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    for frame in data.chunks_mut(sc.channels as usize) {
                        // up-sample
                        let ix = from + frames / 4;
                        if ix < to {
                            let value = cpal::Sample::from::<i16>(&out_copy[ix]);
                            for sample in frame.iter_mut() {
                                *sample = value;
                            }
                        } else if !done {
                            done = true;
                            println!("no more data!");
                            c.wait();
                        }
                        frames += 1;
                    }
                },
                err_fn,
            )?;
            stream.play()?;
            println!("wait..");
            barrier.wait();
            println!("done!");
        }

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
