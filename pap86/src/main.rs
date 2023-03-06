use clap::Parser;
use std::{
    cmp::Ordering,
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

#[derive(Copy, Clone, Debug)]
enum Opcode {
    MovRegToRegOrRegToMem,
    MovImmediateToMem,
    MovImmediateToReg,
    MovMemToAcc,
    MovAccToMem,
}

impl Opcode {
    fn parse(byte: u8) -> Opcode {
        if byte & 0b1111_1100 == 0b1000_1000 {
            return Opcode::MovRegToRegOrRegToMem;
        }

        if byte & 0b1111_1110 == 0b1100_0110 {
            return Opcode::MovImmediateToMem;
        }

        if byte & 0b1111_0000 == 0b1011_0000 {
            return Opcode::MovImmediateToReg;
        }

        if byte & 0b1111_1110 == 0b1010_0000 {
            return Opcode::MovMemToAcc;
        }

        if byte & 0b1111_1110 == 0b1010_0010 {
            return Opcode::MovAccToMem;
        }

        panic!("Invalid opcode: {byte:b}");
    }
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
enum EffectiveAddressFormula {
    BxPlusSi = 0b000,
    BxPlusDi = 0b001,
    BpPlusSi = 0b010,
    BpPlusDi = 0b011,
    Si = 0b100,
    Di = 0b101,
    Bp = 0b110,
    Bx = 0b111,
}

impl Display for EffectiveAddressFormula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectiveAddressFormula::BxPlusSi => write!(f, "bx + si"),
            EffectiveAddressFormula::BxPlusDi => write!(f, "bx + di"),
            EffectiveAddressFormula::BpPlusSi => write!(f, "bp + si"),
            EffectiveAddressFormula::BpPlusDi => write!(f, "bp + di"),
            EffectiveAddressFormula::Si => write!(f, "si"),
            EffectiveAddressFormula::Di => write!(f, "di"),
            EffectiveAddressFormula::Bp => write!(f, "bp"),
            EffectiveAddressFormula::Bx => write!(f, "bx"),
        }
    }
}

fn displacement_str(displacement: &Option<i16>) -> String {
    if let Some(displacement) = displacement {
        match displacement.cmp(&0) {
            Ordering::Greater => format!(" + {displacement}"),
            Ordering::Less => format!(" - {}", displacement.abs()),
            Ordering::Equal => "".to_string(),
        }
    } else {
        "".to_string()
    }
}

#[derive(Copy, Clone, Debug)]
enum Operand {
    Register(Register),
    Mem {
        formula: EffectiveAddressFormula,
        displacement: Option<i16>,
    },
    MemDirect(u16),
    Immediate(u16, bool),
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Register(reg) => write!(f, "{}", reg.as_ref().to_lowercase()),
            Operand::Mem {
                formula,
                displacement,
            } => {
                write!(f, "[{}{}]", formula, displacement_str(displacement),)
            }
            Operand::MemDirect(address) => {
                write!(f, "[{}]", address)
            }
            Operand::Immediate(value, needs_size) => {
                write!(
                    f,
                    "{}",
                    if *needs_size {
                        if *value > 255 {
                            format!("word {value}")
                        } else {
                            format!("byte {value}")
                        }
                    } else {
                        format!("{value}")
                    }
                )
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Instruction {
    Mov { dst: Operand, src: Operand },
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Mov { dst, src } => {
                write!(f, "mov {}, {}", dst, src)
            }
        }
    }
}

struct Input<'a> {
    input: &'a [u8],
    index: usize,
}

impl<'a> Input<'a> {
    fn new(input: &[u8]) -> Input {
        Input { input, index: 0 }
    }

    fn next_byte(&mut self) -> u8 {
        let byte = self.input[self.index];
        self.index += 1;
        byte
    }

    fn next_word(&mut self) -> u16 {
        let lo = self.next_byte() as u16;
        let hi = self.next_byte() as u16;
        (hi << 8) | lo
    }

    fn is_empty(&self) -> bool {
        self.index == self.input.len()
    }
}

fn parse_mem(input: &mut Input, w: u8, instruction_byte_2: u8) -> Result<Operand, String> {
    let mode = instruction_byte_2 >> 6;
    let mem = instruction_byte_2 & 0b111;

    Ok(match mode {
        0b00 => {
            if mem == 0b110 {
                Operand::MemDirect(input.next_word())
            } else {
                Operand::Mem {
                    formula: EffectiveAddressFormula::from_repr(mem)
                        .ok_or_else(|| format!("Invalid formula: {mem:b}"))?,
                    displacement: None,
                }
            }
        }
        0b01 => Operand::Mem {
            formula: EffectiveAddressFormula::from_repr(mem)
                .ok_or_else(|| format!("Invalid formula: {mem:b}"))?,
            displacement: Some(input.next_byte() as i8 as i16),
        },
        0b10 => Operand::Mem {
            formula: EffectiveAddressFormula::from_repr(mem)
                .ok_or_else(|| format!("Invalid formula: {mem:b}"))?,
            displacement: Some(input.next_word() as i16),
        },
        0b11 => {
            let w_reg_2 = (w << 3) | mem;

            Register::from_repr(w_reg_2)
                .map(Operand::Register)
                .ok_or_else(|| format!("Invalid reg: {w_reg_2:b}"))?
        }
        _ => Err("Invalid mode".to_string())?,
    })
}

fn decode(input: &[u8]) -> Vec<Instruction> {
    let mut input = Input::new(input);
    let mut res = Vec::new();

    while !input.is_empty() {
        let instruction_byte_1 = input.next_byte();

        let opcode = Opcode::parse(instruction_byte_1);

        let instruction = match opcode {
            Opcode::MovRegToRegOrRegToMem => {
                let d = (instruction_byte_1 >> 1) & 0b1;
                let w = instruction_byte_1 & 0b1;

                let instruction_byte_2 = input.next_byte();

                let w_reg_1 = (w << 3) | ((instruction_byte_2 >> 3) & 0b111);

                let reg_1 = Register::from_repr(w_reg_1)
                    .map(Operand::Register)
                    .ok_or_else(|| format!("Invalid reg: {w_reg_1:b}"))
                    .unwrap();

                let mem = parse_mem(&mut input, w, instruction_byte_2).unwrap();

                if d > 0 {
                    Instruction::Mov {
                        dst: reg_1,
                        src: mem,
                    }
                } else {
                    Instruction::Mov {
                        dst: mem,
                        src: reg_1,
                    }
                }
            }
            Opcode::MovImmediateToMem => {
                let w = instruction_byte_1 & 0b1;

                let instruction_byte_2 = input.next_byte();

                let mem = parse_mem(&mut input, w, instruction_byte_2).unwrap();

                let data = Operand::Immediate(
                    if w > 0 {
                        input.next_word()
                    } else {
                        input.next_byte() as u16
                    },
                    true,
                );

                Instruction::Mov {
                    dst: mem,
                    src: data,
                }
            }
            Opcode::MovImmediateToReg => {
                let w_reg = instruction_byte_1 & 0b1111;

                let dst = Register::from_repr(w_reg)
                    .map(Operand::Register)
                    .ok_or_else(|| format!("Invalid reg: {w_reg:b}"))
                    .unwrap();

                let data = Operand::Immediate(
                    if w_reg & 0b1000 > 0 {
                        input.next_word()
                    } else {
                        input.next_byte() as u16
                    },
                    false,
                );

                Instruction::Mov { dst, src: data }
            }
            Opcode::MovMemToAcc => {
                let w = instruction_byte_1 & 0b1;

                let addr = Operand::MemDirect(input.next_word());

                Instruction::Mov {
                    dst: Operand::Register(if w > 0 { Register::AX } else { Register::AL }),
                    src: addr,
                }
            }
            Opcode::MovAccToMem => {
                let w = instruction_byte_1 & 0b1;

                let addr = Operand::MemDirect(input.next_word());

                Instruction::Mov {
                    dst: addr,
                    src: Operand::Register(if w > 0 { Register::AX } else { Register::AL }),
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
