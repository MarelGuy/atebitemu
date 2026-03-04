#![allow(clippy::too_many_lines)]
use core::fmt;
use std::error::Error;

use crate::bus::Bus;

enum CpuFlag {
    Carry = 0b0000_0001,
    Zero = 0b0000_0010,
    InterruptDisable = 0b0000_0100,
    Decimal = 0b0000_1000,
    Break = 0b0001_0000,
    Unused = 0b0010_0000,
    Overflow = 0b0100_0000,
    Negative = 0b1000_0000,
}

pub struct Cpu<B: Bus> {
    pub bus: B,
    pub pc: u16, // Program Counter
    sp: u8,      // Stack Pointer
    a: u8,       // Accumulator
    irx: u8,     // Index Register X
    iry: u8,     // Index Register Y
    p: u8, // Processor Status: Carry, Zero, Interrupt Disable, Decimal, Break, Overflow, Negative
    pub process_break: bool,
}

impl<B> fmt::Display for Cpu<B>
where
    B: Bus,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.p;
        let carry = (p & CpuFlag::Carry as u8) != 0;
        let zero = (p & CpuFlag::Zero as u8) != 0;
        let int_disable = (p & CpuFlag::InterruptDisable as u8) != 0;
        let decimal = (p & CpuFlag::Decimal as u8) != 0;
        let break_flag = (p & CpuFlag::Break as u8) != 0;
        let overflow = (p & CpuFlag::Overflow as u8) != 0;
        let negative = (p & CpuFlag::Negative as u8) != 0;

        writeln!(f, "program counter: {:#06X}", self.pc)?;
        writeln!(f, "stack pointer: {:#04X}", self.sp)?;
        writeln!(f, "accumulator register: {:#04X}", self.a)?;
        writeln!(f, "register X: {:#04X}", self.irx)?;
        writeln!(f, "register Y: {:#04X}", self.iry)?;
        writeln!(f, "processor status: {{")?;
        writeln!(f, "   carry: {carry}")?;
        writeln!(f, "   zero: {zero}")?;
        writeln!(f, "   interrupt disable: {int_disable}")?;
        writeln!(f, "   decimal: {decimal}")?;
        writeln!(f, "   break: {break_flag}")?;
        writeln!(f, "   overflow: {overflow}")?;
        writeln!(f, "   negative: {negative}")?;
        write!(f, "}}")
    }
}

impl<B> Cpu<B>
where
    B: Bus,
{
    pub fn new(bus: B) -> Self {
        let mut cpu = Self {
            bus,
            pc: 0,
            sp: 0,
            a: 0,
            irx: 0,
            iry: 0,
            p: 0b0010_0100,
            process_break: false,
        };

        cpu.reset();

        cpu
    }

    pub fn step(&mut self) -> Result<u8, Box<dyn Error>> {
        if self.bus.poll_nmi() {
            self.trigger_nmi();
            self.bus.acknowledge_nmi();

            return Ok(7);
        }

        if self.bus.poll_irq() && (self.p & CpuFlag::InterruptDisable as u8) == 0 {
            self.trigger_irq();

            return Ok(7);
        }

        let opcode = self.bus.read(self.pc);
        self.pc += 1;

        let cc = opcode & 0b11;
        let bbb = (opcode >> 2) & 0b111;
        let aaa = (opcode >> 5) & 0b111;

        let cycles = match cc {
            0b00 => self.zero0(aaa, bbb, cc, opcode),
            0b01 => self.zero1(aaa, bbb, cc, opcode),
            0b10 => self.one0(aaa, bbb, cc, opcode),
            _ => {
                println!("Opcode {:08b} at PC: 0x{:04X}", opcode, self.pc - 1);

                Ok(2)
            }
        }?;

        Ok(cycles)
    }

    // 00
    fn zero0(&mut self, aaa: u8, bbb: u8, cc: u8, opcode: u8) -> Result<u8, Box<dyn Error>> {
        match bbb {
            0b000 => match aaa {
                // BRK
                0b000 => {
                    self.process_break = true;

                    self.push_u16(self.pc.wrapping_add(1));

                    self.push(self.p | CpuFlag::Break as u8 | CpuFlag::Unused as u8);

                    self.set_flag(CpuFlag::Break, true);
                    self.set_flag(CpuFlag::InterruptDisable, true);

                    let lo = u16::from(self.bus.read(0xFFFE));
                    let hi = u16::from(self.bus.read(0xFFFF));

                    self.pc = (hi << 8) | lo;

                    return Ok(7);
                }
                // JSR
                0b001 => {
                    self.push_u16(self.pc.wrapping_add(1));

                    self.pc = self.get_addr();

                    return Ok(6);
                }
                // RTI
                0b010 => {
                    self.p = self.pop();
                    self.pc = self.pop_u16();

                    return Ok(6);
                }
                // RTS
                0b011 => {
                    self.pc = self.pop_u16().wrapping_add(1);

                    return Ok(6);
                }
                _ => {}
            },
            0b010 => {
                let cycles = match aaa {
                    0b000 => {
                        self.push((self.p | CpuFlag::Break as u8) | CpuFlag::Unused as u8);
                        3
                    } // PHP
                    0b001 => {
                        self.p = (self.pop() & !(CpuFlag::Break as u8)) | CpuFlag::Unused as u8;
                        4
                    } // PLP
                    0b010 => {
                        self.push(self.a);
                        3
                    } // PHA
                    0b011 => {
                        self.a = self.pop();

                        self.update_z_and_n_flags(self.a);
                        4
                    } // PLA
                    0b100 => {
                        self.iry = self.iry.wrapping_sub(1);

                        self.update_z_and_n_flags(self.iry);
                        2
                    } // DEY
                    0b101 => {
                        self.iry = self.a;

                        self.update_z_and_n_flags(self.iry);
                        2
                    } // TAY
                    0b110 => {
                        self.iry = self.iry.wrapping_add(1);

                        self.update_z_and_n_flags(self.iry);
                        2
                    } // INY
                    0b111 => {
                        self.irx = self.irx.wrapping_add(1);

                        self.update_z_and_n_flags(self.irx);
                        2
                    } // INX
                    _ => 2,
                };

                return Ok(cycles);
            }
            0b011 => {
                if aaa == 0b011 {
                    // JMP Indirect
                    let ptr = self.get_addr();

                    let lo = u16::from(self.bus.read(ptr));

                    // 6502 page boundary bug
                    let hi = if ptr & 0x00FF == 0x00FF {
                        u16::from(self.bus.read(ptr & 0xFF00))
                    } else {
                        u16::from(self.bus.read(ptr.wrapping_add(1)))
                    };

                    self.pc = (hi << 8) | lo;
                    return Ok(5);
                }
            }
            0b100 => {
                let cycles = match aaa {
                    0b110 | 0b111 => self.branch_if(aaa, 0b110, CpuFlag::Zero), // BNE | BEQ
                    0b000 | 0b001 => self.branch_if(aaa, 0b000, CpuFlag::Negative), // BPL | BMI
                    0b010 | 0b011 => self.branch_if(aaa, 0b010, CpuFlag::Overflow), // BVC | BVS
                    0b100 | 0b101 => self.branch_if(aaa, 0b100, CpuFlag::Carry), // BCC | BCS
                    _ => return Err(format!("Unknown branch: {opcode:08b}").into()),
                };
                return Ok(cycles);
            }
            0b110 => {
                let cycles = match aaa {
                    0b000 => {
                        self.set_flag(CpuFlag::Carry, false);
                        2
                    } // CLC
                    0b001 => {
                        self.set_flag(CpuFlag::Carry, true);
                        2
                    } // SEC
                    0b010 => {
                        self.set_flag(CpuFlag::InterruptDisable, false);
                        2
                    } // CLI
                    0b011 => {
                        self.set_flag(CpuFlag::InterruptDisable, true);
                        2
                    } // SEI
                    0b100 => {
                        self.a = self.iry;
                        self.update_z_and_n_flags(self.a);
                        2
                    } // TYA
                    0b101 => {
                        self.set_flag(CpuFlag::Overflow, false);
                        2
                    } // CLV
                    0b110 => {
                        self.set_flag(CpuFlag::Decimal, false);
                        2
                    } // CLD
                    0b111 => {
                        self.set_flag(CpuFlag::Decimal, true);
                        2
                    } // SED
                    _ => 2,
                };
                return Ok(cycles);
            }
            _ => {}
        }

        let addr = self.get_operand_address_result(aaa, bbb, cc)?;

        let cycles = match aaa {
            // BIT
            0b001 => {
                let mem_val = self.bus.read(addr);

                self.set_flag(CpuFlag::Zero, (mem_val & self.a) == 0);
                self.set_flag(CpuFlag::Negative, mem_val & CpuFlag::Negative as u8 != 0);
                self.set_flag(CpuFlag::Overflow, mem_val & CpuFlag::Overflow as u8 != 0);

                if bbb == 0b001 { 3 } else { 4 }
            }
            // JMP
            0b010 => {
                self.pc = addr;
                3
            }
            // STY
            0b100 => {
                self.bus.write(addr, self.iry);
                if bbb == 0b001 { 3 } else { 4 }
            }
            // LDY
            0b101 => {
                self.iry = self.bus.read(addr);

                self.update_z_and_n_flags(self.iry);

                match bbb {
                    0b000 => 2,
                    0b001 => 3,
                    0b111 => {
                        let base_addr = addr.wrapping_sub(u16::from(self.irx));

                        if (base_addr & 0xFF00) == (addr & 0xFF00) {
                            4
                        } else {
                            5
                        }
                    }
                    _ => 4,
                }
            }
            // CPY
            0b110 => {
                let val = self.bus.read(addr);

                self.compare(self.iry, val);

                match bbb {
                    0b000 => 2,
                    0b001 => 3,
                    _ => 4,
                }
            }
            // CPX
            0b111 => {
                let val = self.bus.read(addr);

                self.compare(self.irx, val);

                match bbb {
                    0b000 => 2,
                    0b001 => 3,
                    _ => 4,
                }
            }
            _ => {
                return Err(
                    format!("opcode {opcode:08b} not implemented in aaa {aaa:03b} 00").into(),
                );
            }
        };

        Ok(cycles)
    }

    // 01
    fn zero1(&mut self, aaa: u8, bbb: u8, cc: u8, opcode: u8) -> Result<u8, Box<dyn Error>> {
        let addr = self.get_operand_address_result(aaa, bbb, cc)?;

        let cycles = match bbb {
            0b000 => 6,         // (Indirect, X)
            0b001 => 3,         // Zero Page
            0b011 | 0b101 => 4, // Absolute | Zero Page, X
            0b100 => {
                // (Indirect), Y
                if aaa == 0b100 {
                    6 // STA always takes 6
                } else {
                    let base_addr = addr.wrapping_sub(u16::from(self.iry));

                    if (base_addr & 0xFF00) == (addr & 0xFF00) {
                        5
                    } else {
                        6
                    }
                }
            }

            0b110 => {
                // Absolute, Y
                if aaa == 0b100 {
                    5 // STA always takes 5
                } else {
                    let base_addr = addr.wrapping_sub(u16::from(self.iry));

                    if (base_addr & 0xFF00) == (addr & 0xFF00) {
                        4
                    } else {
                        5
                    }
                }
            }
            0b111 => {
                // Absolute, X
                if aaa == 0b100 {
                    5 // STA always takes 5
                } else {
                    let base_addr = addr.wrapping_sub(u16::from(self.irx));

                    if (base_addr & 0xFF00) == (addr & 0xFF00) {
                        4
                    } else {
                        5
                    }
                }
            }
            _ => 2,
        };

        match aaa {
            // ORA
            0b000 => {
                self.a |= self.bus.read(addr);

                self.update_z_and_n_flags(self.a);
            }
            // AND
            0b001 => {
                self.a &= self.bus.read(addr);

                self.update_z_and_n_flags(self.a);
            }
            // EOR
            0b010 => {
                self.a ^= self.bus.read(addr);

                self.update_z_and_n_flags(self.a);
            }
            // ADC | SBC
            0b011 | 0b111 => {
                let is_sbc = aaa == 0b111;
                let mem_val = self.bus.read(addr);
                let carry_in = u16::from(self.p & CpuFlag::Carry as u8 != 0);

                let bin_value = if is_sbc { mem_val ^ 0xFF } else { mem_val };
                let sum = u16::from(self.a) + u16::from(bin_value) + carry_in;

                #[allow(clippy::cast_possible_truncation)]
                let bin_result = sum as u8;

                self.set_flag(CpuFlag::Zero, bin_result == 0);
                self.set_flag(CpuFlag::Negative, bin_result & 0x80 != 0);
                self.set_flag(
                    CpuFlag::Overflow,
                    (self.a ^ bin_result) & (bin_value ^ bin_result) & 0x80 != 0,
                );

                if self.p & CpuFlag::Decimal as u8 != 0 {
                    #[allow(clippy::cast_possible_truncation)]
                    let c_in = carry_in as u8;

                    if is_sbc {
                        let mut lo = (self.a & 0x0F)
                            .wrapping_sub(mem_val & 0x0F)
                            .wrapping_sub(1 - c_in);
                        let mut hi = (self.a >> 4)
                            .wrapping_sub(mem_val >> 4)
                            .wrapping_sub(u8::from((lo & 0x10) != 0));

                        if (lo & 0x10) != 0 {
                            lo = lo.wrapping_sub(0x06);
                        }
                        if (hi & 0x10) != 0 {
                            hi = hi.wrapping_sub(0x06);
                        }

                        self.set_flag(CpuFlag::Carry, (hi & 0x10) == 0);

                        self.a = (lo & 0x0F) | ((hi << 4) & 0xF0);
                    } else {
                        let mut lo = (self.a & 0x0F) + (mem_val & 0x0F) + c_in;
                        let mut hi = (self.a >> 4) + (mem_val >> 4) + u8::from(lo > 0x09);

                        if lo > 0x09 {
                            lo += 0x06;
                        }
                        if hi > 0x09 {
                            hi += 0x06;
                        }

                        self.set_flag(CpuFlag::Carry, hi > 0x0F);

                        self.a = (lo & 0x0F) | ((hi << 4) & 0xF0);
                    }
                } else {
                    self.set_flag(CpuFlag::Carry, sum > 0xFF);

                    self.a = bin_result;
                }
            }
            // STA
            0b100 => {
                self.bus.write(addr, self.a);
            }
            // LDA
            0b101 => {
                self.a = self.bus.read(addr);

                self.update_z_and_n_flags(self.a);
            }
            // CMP
            0b110 => {
                let val = self.bus.read(addr);

                self.compare(self.a, val);
            }
            _ => {
                return Err(format!(
                    "Group 1 operation {opcode:08b} not implemented in aaa {aaa:03b}"
                )
                .into());
            }
        }

        Ok(cycles)
    }

    // 10
    fn one0(&mut self, aaa: u8, bbb: u8, cc: u8, opcode: u8) -> Result<u8, Box<dyn Error>> {
        let addr = self.get_operand_address(aaa, bbb, cc);
        let cycles;

        match aaa {
            // ASL
            0b000 => {
                if bbb == 0b010 {
                    // Accumulator mode
                    self.set_flag(CpuFlag::Carry, self.a & 0x80 != 0);

                    self.a <<= 1;

                    self.update_z_and_n_flags(self.a);
                    cycles = 2;
                } else {
                    // Memory mode
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for ASL: {opcode:08b}"))?;

                    let mut value = self.bus.read(valid_addr);
                    self.set_flag(CpuFlag::Carry, value & 0x80 != 0);

                    value <<= 1;

                    self.bus.write(valid_addr, value);
                    self.update_z_and_n_flags(value);

                    cycles = match bbb {
                        0b001 => 5, // Zero Page
                        0b111 => 7, // Absolute, X
                        _ => 6,
                    };
                }
            }
            // ROL
            0b001 => {
                let old_carry = u8::from(self.p & CpuFlag::Carry as u8 != 0);

                if bbb == 0b010 {
                    // Accumulator mode
                    self.set_flag(CpuFlag::Carry, self.a & CpuFlag::Negative as u8 != 0);
                    self.a = (self.a << 1) | old_carry;
                    self.update_z_and_n_flags(self.a);
                    cycles = 2;
                } else {
                    // Memory mode
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for ROL: {opcode:08b}"))?;

                    let mut value = self.bus.read(valid_addr);
                    self.set_flag(CpuFlag::Carry, value & CpuFlag::Negative as u8 != 0);

                    value = (value << 1) | old_carry;

                    self.bus.write(valid_addr, value);
                    self.update_z_and_n_flags(value);

                    cycles = match bbb {
                        0b001 => 5,
                        0b111 => 7,
                        _ => 6,
                    };
                }
            }
            // LSR
            0b010 => {
                if bbb == 0b010 {
                    // Accumulator mode
                    self.set_flag(CpuFlag::Carry, self.a & 0x01 != 0);

                    self.a >>= 1;

                    self.update_z_and_n_flags(self.a);
                    cycles = 2;
                } else {
                    // Memory addressing modes
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for LSR: {opcode:08b}"))?;

                    let mut value = self.bus.read(valid_addr);

                    self.set_flag(CpuFlag::Carry, value & 0x01 != 0);

                    value >>= 1;

                    self.bus.write(valid_addr, value);
                    self.update_z_and_n_flags(value);

                    cycles = match bbb {
                        0b001 => 5,
                        0b111 => 7,
                        _ => 6,
                    };
                }
            }
            // ROR
            0b011 => {
                let old_carry = u8::from(self.p & CpuFlag::Carry as u8 != 0);

                if bbb == 0b010 {
                    // Accumulator mode
                    self.set_flag(CpuFlag::Carry, self.a & 0x01 != 0);

                    self.a = (self.a >> 1) | (old_carry << 7);

                    self.update_z_and_n_flags(self.a);
                    cycles = 2;
                } else {
                    // Memory mode
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for ROR: {opcode:08b}"))?;

                    let mut value = self.bus.read(valid_addr);

                    self.set_flag(CpuFlag::Carry, value & 0x01 != 0);

                    value = (value >> 1) | (old_carry << 7);

                    self.bus.write(valid_addr, value);
                    self.update_z_and_n_flags(value);

                    cycles = match bbb {
                        0b001 => 5,
                        0b111 => 7,
                        _ => 6,
                    };
                }
            }
            0b100 => {
                if bbb == 0b010 {
                    // TXA (Implied)
                    self.a = self.irx;

                    self.update_z_and_n_flags(self.a);

                    cycles = 2;
                } else if bbb == 0b110 {
                    // TXS (Implied)
                    self.sp = self.irx;
                    cycles = 2;
                } else {
                    // STX (Zero Page, Zero Page Y, Absolute)
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for STX: {opcode:08b}"))?;

                    self.bus.write(valid_addr, self.irx);

                    cycles = match bbb {
                        0b001 => 3, // Zero Page
                        _ => 4,
                    };
                }
            }
            // TAX / TSX / LDX
            0b101 => {
                if bbb == 0b010 {
                    // TAX
                    self.irx = self.a;

                    self.update_z_and_n_flags(self.irx);
                    cycles = 2;
                } else if bbb == 0b110 {
                    // TSX
                    self.irx = self.sp;

                    self.update_z_and_n_flags(self.irx);
                    cycles = 2;
                } else {
                    // LDX
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for LDX: {opcode:02b}"))?;

                    self.irx = self.bus.read(valid_addr);

                    self.update_z_and_n_flags(self.irx);

                    cycles = match bbb {
                        0b000 => 2, // Immediate
                        0b001 => 3, // Zero Page
                        0b111 => {
                            // Absolute, Y - Page crossing logic
                            let base_addr = valid_addr.wrapping_sub(u16::from(self.iry));
                            if (base_addr & 0xFF00) == (valid_addr & 0xFF00) {
                                4
                            } else {
                                5
                            }
                        }
                        _ => 4,
                    };
                }
            }
            0b110 => {
                // DEX
                if bbb == 0b010 {
                    self.irx = self.irx.wrapping_sub(1);
                    self.update_z_and_n_flags(self.irx);
                    cycles = 2;
                // DEC
                } else {
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for DEC: {opcode:02b}"))?;

                    let result = self.bus.read(valid_addr).wrapping_sub(1);
                    self.bus.write(valid_addr, result);
                    self.update_z_and_n_flags(result);

                    cycles = match bbb {
                        0b001 => 5,
                        0b111 => 7,
                        _ => 6,
                    };
                }
            }
            // INC
            0b111 => {
                if bbb == 0b010 {
                    // NOP (0xEA)
                    cycles = 2;
                } else {
                    // INC
                    let valid_addr = addr
                        .ok_or_else(|| format!("Illegal addressing mode for INC: {opcode:02b}"))?;

                    let result = self.bus.read(valid_addr).wrapping_add(1);
                    self.bus.write(valid_addr, result);
                    self.update_z_and_n_flags(result);

                    cycles = match bbb {
                        0b001 => 5,
                        0b111 => 7,
                        _ => 6,
                    };
                }
            }
            _ => {
                return Err(format!("opcode {opcode:08b} not implemented in aaa {bbb} 10").into());
            }
        }

        Ok(cycles)
    }

    fn branch_if(&mut self, aaa: u8, target_aaa: u8, flag: CpuFlag) -> u8 {
        let offset = self.get_immediate().cast_signed();
        let is_flag_clear = self.p & (flag as u8) == 0;

        let mut cycles = 2;

        if (is_flag_clear && aaa == target_aaa) || (!is_flag_clear && aaa == target_aaa + 1) {
            cycles += 1;

            let old_pc = self.pc;

            self.pc = self.pc.wrapping_add_signed(i16::from(offset));

            if (old_pc & 0xFF00) != (self.pc & 0xFF00) {
                cycles += 1;
            }
        }

        cycles
    }

    // Hardware NMI (triggered by PPU VBlank)
    pub fn trigger_nmi(&mut self) {
        self.push_u16(self.pc);
        self.push((self.p & !(CpuFlag::Break as u8)) | CpuFlag::Unused as u8);
        self.set_flag(CpuFlag::InterruptDisable, true);

        let lo = u16::from(self.bus.read(0xFFFA));
        let hi = u16::from(self.bus.read(0xFFFB));

        self.pc = (hi << 8) | lo;
    }

    // Hardware IRQ (triggered by APU or Mappers)
    pub fn trigger_irq(&mut self) {
        if (self.p & CpuFlag::InterruptDisable as u8) == 0 {
            self.push_u16(self.pc);
            self.push((self.p & !(CpuFlag::Break as u8)) | CpuFlag::Unused as u8);
            self.set_flag(CpuFlag::InterruptDisable, true);

            let lo = u16::from(self.bus.read(0xFFFE));
            let hi = u16::from(self.bus.read(0xFFFF));

            self.pc = (hi << 8) | lo;
        }
    }

    fn compare(&mut self, reg: u8, val: u8) {
        self.set_flag(CpuFlag::Carry, reg >= val);

        self.update_z_and_n_flags(reg.wrapping_sub(val));
    }

    fn push(&mut self, val: u8) {
        self.bus.write(0x0100u16 + u16::from(self.sp), val);

        self.sp = self.sp.wrapping_sub(1);
    }

    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);

        self.bus.read(0x0100u16 + u16::from(self.sp))
    }

    fn push_u16(&mut self, val: u16) {
        let hi = (val >> 8) as u8;
        let lo = (val & 0xFF) as u8;

        self.push(hi);
        self.push(lo);
    }

    fn pop_u16(&mut self) -> u16 {
        let lo = u16::from(self.pop());
        let hi = u16::from(self.pop());

        (hi << 8) | lo
    }

    fn get_operand_address_result(
        &mut self,
        aaa: u8,
        bbb: u8,
        cc: u8,
    ) -> Result<u16, Box<dyn Error>> {
        self.get_operand_address(aaa, bbb, cc).ok_or_else(|| {
            format!("Illegal addressing mode for: {aaa:03b}, {bbb:03b}, {cc:02b}").into()
        })
    }

    fn get_operand_address(&mut self, aaa: u8, bbb: u8, cc: u8) -> Option<u16> {
        match bbb {
            0b000 => {
                if cc == 0b01 {
                    // (Indirect, X) for Group 1
                    let ptr = self.get_immediate().wrapping_add(self.irx);

                    let lo = u16::from(self.bus.read(u16::from(ptr)));
                    let hi = u16::from(self.bus.read(u16::from(ptr.wrapping_add(1))));

                    Some((hi << 8) | lo)
                } else {
                    // Immediate (LDX) and (LDY, CPX, CPY)
                    let addr = self.pc;

                    self.pc += 1;

                    Some(addr)
                }
            }
            0b001 => {
                // Zero Page (all groups)
                let addr = u16::from(self.get_immediate());

                Some(addr)
            }
            0b010 => {
                if cc == 0b01 {
                    // Immediate for Group 1
                    let addr = self.pc;

                    self.pc += 1;

                    Some(addr)
                } else {
                    // Accumulator or Implied for Groups 2 and 3
                    None
                }
            }
            0b011 => {
                // Absolute (all groups)
                Some(self.get_addr())
            }
            0b100 => {
                if cc == 0b01 {
                    // (Indirect), Y for Group 1
                    let ptr = self.get_immediate();

                    let lo = u16::from(self.bus.read(u16::from(ptr)));
                    let hi = u16::from(self.bus.read(u16::from(ptr.wrapping_add(1))));

                    Some(((hi << 8) | lo).wrapping_add(u16::from(self.iry)))
                } else {
                    // Relative / Implied fallbacks
                    let addr = self.pc;

                    self.pc += 1;

                    Some(addr)
                }
            }
            0b101 => {
                // Zero Page, X (default) OR Zero Page, Y for STX/LDX in Group 2
                let offset = if cc == 0b10 && (aaa == 0b100 || aaa == 0b101) {
                    self.iry
                } else {
                    self.irx
                };

                Some(u16::from(self.get_immediate().wrapping_add(offset)))
            }
            0b110 => {
                if cc == 0b01 {
                    // Absolute, Y for Group 1
                    Some(self.get_addr().wrapping_add(u16::from(self.iry)))
                } else {
                    // Implied
                    None
                }
            }
            0b111 => {
                // Absolute, X (default) OR Absolute, Y for LDX in Group 2
                let offset = if cc == 0b10 && aaa == 0b101 {
                    self.iry
                } else {
                    self.irx
                };

                Some(self.get_addr().wrapping_add(u16::from(offset)))
            }
            _ => None,
        }
    }

    fn get_addr(&mut self) -> u16 {
        u16::from(self.get_immediate()) | (u16::from(self.get_immediate()) << 8)
    }

    fn get_immediate(&mut self) -> u8 {
        let value = self.bus.read(self.pc);

        self.pc += 1;

        value
    }

    fn update_z_and_n_flags(&mut self, result: u8) {
        self.set_flag(CpuFlag::Zero, result == 0);
        self.set_flag(CpuFlag::Negative, result & CpuFlag::Negative as u8 != 0);
    }

    fn set_flag(&mut self, flag: CpuFlag, high: bool) {
        let mask = flag as u8;

        if high {
            self.p |= mask;
        } else {
            self.p &= !mask;
        }
    }

    fn reset(&mut self) {
        let low = u16::from(self.bus.read(0xFFFC));
        let high = u16::from(self.bus.read(0xFFFD));

        self.pc = (high << 8) | low;
        self.sp = 0xFD;
        self.p = 0b0010_0100;
    }
}
