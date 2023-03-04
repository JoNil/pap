use std::fs;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to disassemble
    file: String,
}

// Register from encoding W | REG
#[derive(Debug)]
enum Register {
    AL = 0b0000,
    CL = 0b0001,
    DL = 0b0010,
    BL = 0b0011,
    AH = 0b0100,
    CH = 0b0101,
    DH = 0b0110,
    BH = 0b0111,
    AX = 0b1000,
    CX = 0b1001,
    DX = 0b1010,
    BX = 0b1011,
    SP = 0b1100,
    BP = 0b1101,
    SI = 0b1110,
    DI = 0b1111,
}

enum Opcode {
    Mov = 0b100010,
}

#[derive(Debug)]
enum Instruction {
    MovRegReg { from: Register, to: Register },
}

fn decode(input: &[u8]) -> Vec<Instruction> {
    let mut res = Vec::new();

    res
}

fn main() {
    let cli = Args::parse();

    let input = fs::read(&cli.file)
        .map_err(|e| panic!("Unable to read {}: {e:?}", &cli.file))
        .unwrap();

    let instructions = decode(&input);

    println!("{instructions:?}")
}
