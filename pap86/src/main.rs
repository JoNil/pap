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

    fn is_empty(&self) -> bool {
        self.index == self.input.len()
    }
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

                let Some(reg_1) = Register::from_repr(w_reg_1).map(Operand::Register) else {
                    panic!("Invalid reg: {w_reg_1:b}")
                };

                let mode = instruction_byte_2 >> 6;

                match mode {
                    0b00 => {
                        let mem = instruction_byte_2 & 0b111;

                        if mem == 0b110 {
                            let direct = Operand::MemDirect({
                                let instruction_byte_2 = input.next_byte();
                                let instruction_byte_3 = input.next_byte();
                                ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                            });

                            if d > 0 {
                                Instruction::Mov {
                                    dst: reg_1,
                                    src: direct,
                                }
                            } else {
                                Instruction::Mov {
                                    dst: direct,
                                    src: reg_1,
                                }
                            }
                        } else {
                            let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                                panic!("Invalid formula: {mem:b}");
                            };

                            let mem = Operand::Mem {
                                formula,
                                displacement: None,
                            };

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
                    }
                    0b01 => {
                        let mem = instruction_byte_2 & 0b111;

                        let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                            panic!("Invalid formula: {mem:b}");
                        };

                        let displacement = input.next_byte() as i8 as i16;

                        let mem = Operand::Mem {
                            formula,
                            displacement: Some(displacement),
                        };

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
                    0b10 => {
                        let mem = instruction_byte_2 & 0b111;

                        let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                            panic!("Invalid formula: {mem:b}");
                        };

                        let displacement = {
                            let instruction_byte_2 = input.next_byte();
                            let instruction_byte_3 = input.next_byte();
                            ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                        } as i16;

                        let mem = Operand::Mem {
                            formula,
                            displacement: Some(displacement),
                        };

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
                    0b11 => {
                        let w_reg_2 = (w << 3) | (instruction_byte_2 & 0b111);

                        let Some(reg_2) = Register::from_repr(w_reg_2).map(Operand::Register) else {
                            panic!("Invalid reg: {w_reg_2:b}")
                        };

                        let dst = if d == 0b1 { reg_1 } else { reg_2 };
                        let src = if d == 0b1 { reg_2 } else { reg_1 };

                        Instruction::Mov { dst, src }
                    }
                    _ => {
                        panic!("Invalid mode!");
                    }
                }
            }
            Opcode::MovImmediateToMem => {
                let w = instruction_byte_1 & 0b1;

                let instruction_byte_2 = input.next_byte();

                let mode = instruction_byte_2 >> 6;

                let get_data = |input: &mut Input| {
                    Operand::Immediate(
                        if w > 0 {
                            let lo = input.next_byte();
                            let hi = input.next_byte();
                            ((hi as u16) << 8) | (lo as u16)
                        } else {
                            input.next_byte() as u16
                        },
                        true,
                    )
                };

                match mode {
                    0b00 => {
                        let mem = instruction_byte_2 & 0b111;

                        if mem == 0b110 {
                            let direct = Operand::MemDirect({
                                let instruction_byte_2 = input.next_byte();
                                let instruction_byte_3 = input.next_byte();
                                ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                            });

                            let data = get_data(&mut input);

                            Instruction::Mov {
                                dst: direct,
                                src: data,
                            }
                        } else {
                            let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                                panic!("Invalid formula: {mem:b}");
                            };

                            let mem = Operand::Mem {
                                formula,
                                displacement: None,
                            };

                            let data = get_data(&mut input);

                            Instruction::Mov {
                                dst: mem,
                                src: data,
                            }
                        }
                    }
                    0b01 => {
                        let mem = instruction_byte_2 & 0b111;

                        let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                            panic!("Invalid formula: {mem:b}");
                        };

                        let displacement = input.next_byte() as i8 as i16;

                        let mem = Operand::Mem {
                            formula,
                            displacement: Some(displacement),
                        };

                        let data = get_data(&mut input);

                        Instruction::Mov {
                            dst: mem,
                            src: data,
                        }
                    }
                    0b10 => {
                        let mem = instruction_byte_2 & 0b111;

                        let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                            panic!("Invalid formula: {mem:b}");
                        };

                        let displacement = {
                            let instruction_byte_2 = input.next_byte();
                            let instruction_byte_3 = input.next_byte();
                            ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                        } as i16;

                        let mem = Operand::Mem {
                            formula,
                            displacement: Some(displacement),
                        };

                        let data = get_data(&mut input);

                        Instruction::Mov {
                            dst: mem,
                            src: data,
                        }
                    }
                    0b11 => {
                        let w_reg = (w << 3) | (instruction_byte_2 & 0b111);

                        let Some(reg) = Register::from_repr(w_reg).map(Operand::Register) else {
                            panic!("Invalid reg: {w_reg:b}")
                        };

                        let data = get_data(&mut input);

                        Instruction::Mov {
                            dst: reg,
                            src: data,
                        }
                    }
                    _ => {
                        panic!("Invalid mode!");
                    }
                }
            }
            Opcode::MovImmediateToReg => {
                let w_reg = instruction_byte_1 & 0b1111;

                let Some(dst) = Register::from_repr(w_reg).map(Operand::Register) else {
                    panic!("Invalid reg: {w_reg:b}")
                };

                let data = Operand::Immediate(
                    if w_reg & 0b1000 > 0 {
                        let instruction_byte_2 = input.next_byte();
                        let instruction_byte_3 = input.next_byte();
                        ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                    } else {
                        input.next_byte() as u16
                    },
                    false,
                );

                Instruction::Mov { dst, src: data }
            }
            Opcode::MovMemToAcc => {
                let w = instruction_byte_1 & 0b1;

                let addr = Operand::MemDirect({
                    let instruction_byte_2 = input.next_byte();
                    let instruction_byte_3 = input.next_byte();
                    ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                });

                Instruction::Mov {
                    dst: Operand::Register(if w > 0 { Register::AX } else { Register::AL }),
                    src: addr,
                }
            }
            Opcode::MovAccToMem => {
                let w = instruction_byte_1 & 0b1;

                let addr = Operand::MemDirect({
                    let instruction_byte_2 = input.next_byte();
                    let instruction_byte_3 = input.next_byte();
                    ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                });

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
