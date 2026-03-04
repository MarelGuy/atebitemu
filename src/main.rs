use clap::Parser;
use std::{
    error::Error,
    fs,
    time::{Duration, Instant},
};

mod bus;
mod cpu;
mod memory;

use crate::{cpu::Cpu, memory::Memory};

/// A custom parser for hex addresses (like 0x0400 or just 0400)
fn parse_hex(src: &str) -> Result<u16, std::num::ParseIntError> {
    u16::from_str_radix(src.trim_start_matches("0x"), 16)
}

#[derive(Parser, Debug)]
#[command(name = "atebitemu")]
#[command(version, long_about = None)]
struct Args {
    /// Path to the binary program to load
    program_path: String,

    /// Run the emulator without a clock speed limit
    #[arg(short, long)]
    unlimited: bool,

    /// CPU clock speed in MHz
    #[arg(short, long, default_value_t = 1)]
    mhz: usize,

    /// Print CPU state after every instruction (Trace mode)
    #[arg(short, long)]
    trace: bool,

    /// Override the starting Program Counter (PC) address (in hex)
    #[arg(short, long, value_parser = parse_hex)]
    start_pc: Option<u16>,
}

fn run(mut cpu: Cpu<Memory>, args: &Args) -> Result<(), Box<dyn Error>> {
    let clock_speed_hz = args.mhz * 1_000_000;
    let batch_duration = Duration::from_millis(16);
    let cycles_per_batch = (clock_speed_hz * 16) / 1000;

    let mut next_batch_time = Instant::now() + batch_duration;
    let mut total_cycles = 0;

    loop {
        let mut cycles_this_batch = 0;

        while cycles_this_batch < cycles_per_batch {
            let old_pc = cpu.pc;

            if args.trace {
                println!("{cpu}");
            }

            let step_cycles = usize::from(cpu.step()?);

            if cpu.pc == old_pc {
                println!("\nInfinite loop trap");
                println!("{cpu}");
                println!("Total cycles: {}", total_cycles + step_cycles);

                return Ok(());
            }

            cycles_this_batch += step_cycles;
            total_cycles += step_cycles;
        }

        if !args.unlimited {
            let now = Instant::now();

            if next_batch_time > now {
                std::thread::sleep(next_batch_time - now);
            } else {
                next_batch_time = now;
            }

            next_batch_time += batch_duration;
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let program = fs::read(&args.program_path)?;

    let mem = memory::Memory::new(&program);
    let mut cpu = cpu::Cpu::new(mem);

    if let Some(pc) = args.start_pc {
        cpu.pc = pc;
    }

    run(cpu, &args)?;

    Ok(())
}
