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

#[derive(Copy, Clone, Debug)]
enum Opcode {
    MovRegToRegOrRegToMem,
    MovImmediateToMem,
    MovImmediateToReg,
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

        panic!("Invalid opcode: {byte:b}");
    }
}

#[derive(Copy, Clone, Debug)]
enum Instruction {
    MovRegToReg {
        dst: Register,
        src: Register,
    },
    MovMemToReg {
        dst: Register,
        formula: EffectiveAddressFormula,
        displacement: Option<u16>,
    },
    MovRegToMem {
        formula: EffectiveAddressFormula,
        displacement: Option<u16>,
        src: Register,
    },
    MovMemDirectToReg {
        dst: Register,
        address: u16,
    },
    MovRegToMemDirect {
        address: u16,
        src: Register,
    },

    MovImmediateToMem {
        formula: EffectiveAddressFormula,
        displacement: Option<u16>,
        data: u16,
    },
    MovImmediateMemDirect {
        address: u16,
        data: u16,
    },

    MovImmediateToReg {
        dst: Register,
        data: u16,
    },
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::MovRegToReg { dst, src } => {
                write!(
                    f,
                    "mov {}, {}",
                    dst.as_ref().to_lowercase(),
                    src.as_ref().to_lowercase()
                )
            }
            Instruction::MovMemToReg {
                dst,
                formula,
                displacement,
            } => {
                write!(
                    f,
                    "mov {}, [{}{}]",
                    dst.as_ref().to_lowercase(),
                    formula,
                    if let Some(displacement) = displacement {
                        if *displacement > 0 {
                            format!(" + {displacement}")
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    }
                )
            }
            Instruction::MovRegToMem {
                formula,
                displacement,
                src,
            } => {
                write!(
                    f,
                    "mov [{}{}], {}",
                    formula,
                    if let Some(displacement) = displacement {
                        if *displacement > 0 {
                            format!(" + {displacement}")
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    },
                    src.as_ref().to_lowercase(),
                )
            }
            Instruction::MovMemDirectToReg { dst, address } => {
                write!(f, "mov {}, [{}]", dst.as_ref().to_lowercase(), address)
            }
            Instruction::MovRegToMemDirect { address, src } => {
                write!(f, "mov [{}], {}", address, src.as_ref().to_lowercase())
            }

            Instruction::MovImmediateToMem {
                formula,
                displacement,
                data,
            } => {
                write!(
                    f,
                    "mov [{}{}], {}",
                    formula,
                    if let Some(displacement) = displacement {
                        if *displacement > 0 {
                            format!(" + {displacement}")
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    },
                    if *data > 255 {
                        format!("word {data}")
                    } else {
                        format!("byte {data}")
                    },
                )
            }
            Instruction::MovImmediateMemDirect { address, data } => {
                write!(
                    f,
                    "mov [{}], {}",
                    address,
                    if *data > 255 {
                        format!("word {data}")
                    } else {
                        format!("byte {data}")
                    }
                )
            }

            Instruction::MovImmediateToReg { dst, data } => {
                write!(f, "mov {}, {}", dst.as_ref().to_lowercase(), data)
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

                let Some(reg_1) = Register::from_repr(w_reg_1) else {
                    panic!("Invalid reg: {w_reg_1:b}")
                };

                let mode = instruction_byte_2 >> 6;

                match mode {
                    0b00 => {
                        let mem = instruction_byte_2 & 0b111;

                        if mem == 0b110 {
                            let direct = {
                                let instruction_byte_2 = input.next_byte();
                                let instruction_byte_3 = input.next_byte();
                                ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                            };

                            if d > 0 {
                                Instruction::MovMemDirectToReg {
                                    dst: reg_1,
                                    address: direct,
                                }
                            } else {
                                Instruction::MovRegToMemDirect {
                                    address: direct,
                                    src: reg_1,
                                }
                            }
                        } else {
                            let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                                panic!("Invalid formula: {mem:b}");
                            };

                            if d > 0 {
                                Instruction::MovMemToReg {
                                    dst: reg_1,
                                    formula,
                                    displacement: None,
                                }
                            } else {
                                Instruction::MovRegToMem {
                                    formula,
                                    displacement: None,
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

                        let displacement = input.next_byte() as u16;

                        if d > 0 {
                            Instruction::MovMemToReg {
                                dst: reg_1,
                                formula,
                                displacement: Some(displacement),
                            }
                        } else {
                            Instruction::MovRegToMem {
                                formula,
                                displacement: Some(displacement),
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
                        };

                        if d > 0 {
                            Instruction::MovMemToReg {
                                dst: reg_1,
                                formula,
                                displacement: Some(displacement),
                            }
                        } else {
                            Instruction::MovRegToMem {
                                formula,
                                displacement: Some(displacement),
                                src: reg_1,
                            }
                        }
                    }
                    0b11 => {
                        let w_reg_2 = (w << 3) | (instruction_byte_2 & 0b111);

                        let Some(reg_2) = Register::from_repr(w_reg_2) else {
                            panic!("Invalid reg: {w_reg_2:b}")
                        };

                        let dst = if d == 0b1 { reg_1 } else { reg_2 };
                        let src = if d == 0b1 { reg_2 } else { reg_1 };

                        Instruction::MovRegToReg { dst, src }
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
                    if w > 0 {
                        let lo = input.next_byte();
                        let hi = input.next_byte();
                        ((hi as u16) << 8) | (lo as u16)
                    } else {
                        input.next_byte() as u16
                    }
                };

                match mode {
                    0b00 => {
                        let mem = instruction_byte_2 & 0b111;

                        if mem == 0b110 {
                            let direct = {
                                let instruction_byte_2 = input.next_byte();
                                let instruction_byte_3 = input.next_byte();
                                ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                            };

                            let data = get_data(&mut input);

                            Instruction::MovImmediateMemDirect {
                                address: direct,
                                data,
                            }
                        } else {
                            let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                                panic!("Invalid formula: {mem:b}");
                            };

                            let data = get_data(&mut input);

                            Instruction::MovImmediateToMem {
                                formula,
                                displacement: None,
                                data,
                            }
                        }
                    }
                    0b01 => {
                        let mem = instruction_byte_2 & 0b111;

                        let Some(formula) = EffectiveAddressFormula::from_repr(mem) else {
                            panic!("Invalid formula: {mem:b}");
                        };

                        let displacement = input.next_byte() as u16;

                        let data = get_data(&mut input);

                        Instruction::MovImmediateToMem {
                            formula,
                            displacement: Some(displacement),
                            data,
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
                        };

                        let data = get_data(&mut input);

                        Instruction::MovImmediateToMem {
                            formula,
                            displacement: Some(displacement),
                            data,
                        }
                    }
                    0b11 => {
                        let w_reg = (w << 3) | (instruction_byte_2 & 0b111);

                        let Some(reg) = Register::from_repr(w_reg) else {
                            panic!("Invalid reg: {w_reg:b}")
                        };

                        let data = get_data(&mut input);

                        Instruction::MovImmediateToReg { dst: reg, data }
                    }
                    _ => {
                        panic!("Invalid mode!");
                    }
                }
            }
            Opcode::MovImmediateToReg => {
                let w_reg = instruction_byte_1 & 0b1111;

                let Some(dst) = Register::from_repr(w_reg) else {
                    panic!("Invalid reg: {w_reg:b}")
                };

                let data = if w_reg & 0b1000 > 0 {
                    let instruction_byte_2 = input.next_byte();
                    let instruction_byte_3 = input.next_byte();
                    ((instruction_byte_3 as u16) << 8) | (instruction_byte_2 as u16)
                } else {
                    input.next_byte() as u16
                };

                Instruction::MovImmediateToReg { dst, data }
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
