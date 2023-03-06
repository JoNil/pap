use clap::Parser;
use std::{
    fmt::Display,
    fs::{self, File},
    io::Write,
};
use strum_macros::{AsRefStr, FromRepr};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to disassemble
    file: String,

    /// Output file
    #[arg(long, short)]
    output: Option<String>,
}

// Register from encoding W | REG
#[derive(AsRefStr, Copy, Clone, Debug, FromRepr)]
#[repr(u8)]
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

#[derive(Copy, Clone, Debug, FromRepr)]
#[repr(u8)]
enum Opcode {
    Mov = 0b100010,
}

#[derive(Copy, Clone, Debug)]
enum Instruction {
    MovRegReg { dst: Register, src: Register },
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::MovRegReg { dst, src } => {
                write!(
                    f,
                    "mov {}, {}",
                    dst.as_ref().to_lowercase(),
                    src.as_ref().to_lowercase()
                )
            }
        }
    }
}

fn decode(input: &[u8]) -> Vec<Instruction> {
    let mut res = Vec::new();

    let mut index = 0;

    while index < input.len() {
        let instruction_byte_1 = input[index];
        index += 1;

        let opcode = instruction_byte_1 >> 2;
        let Some(opcode) = Opcode::from_repr(opcode) else {
            panic!("Invalid Opcode: {opcode:b}")
        };

        let instruction = match opcode {
            Opcode::Mov => {
                let d = (instruction_byte_1 >> 1) & 0b1;
                let w = instruction_byte_1 & 0b1;

                let instruction_byte_2 = input[index];
                index += 1;

                let mode = instruction_byte_2 >> 6;

                match mode {
                    0b11 => {
                        let w_reg_1 = (w << 3) | ((instruction_byte_2 >> 3) & 0b111);
                        let w_reg_2 = (w << 3) | (instruction_byte_2 & 0b111);

                        let Some(reg_1) = Register::from_repr(w_reg_1) else {
                            panic!("Invalid reg: {w_reg_1:b}")
                        };

                        let Some(reg_2) = Register::from_repr(w_reg_2) else {
                            panic!("Invalid reg: {w_reg_2:b}")
                        };

                        let dst = if d == 0b1 { reg_1 } else { reg_2 };
                        let src = if d == 0b1 { reg_2 } else { reg_1 };

                        Instruction::MovRegReg { dst, src }
                    }
                    _ => {
                        panic!("Unsupported mode!");
                    }
                }
            }
        };

        res.push(instruction);
    }

    res
}

fn output(w: &mut dyn Write, instructions: &[Instruction]) {
    writeln!(w, "bits 16").unwrap();
    for instruction in instructions {
        writeln!(w, "{instruction}").unwrap();
    }
}

fn main() {
    let cli = Args::parse();

    let input = fs::read(&cli.file)
        .map_err(|e| panic!("Unable to read {}: {e:?}", &cli.file))
        .unwrap();

    let instructions = decode(&input);

    if let Some(file) = cli.output {
        let mut file = File::create(file).unwrap();
        output(&mut file, &instructions);
    } else {
        output(&mut std::io::stdout(), &instructions);
    };
}
