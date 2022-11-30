use crate::opcodes;
use std::collections::HashMap;

bitflags! {
    pub struct CpuFlags: u8 {
        const CARRY             = 0b00000001;
        const ZERO              = 0b00000010;
        const INTERRUPT_DISABLE = 0b00000100;
        const DECIMAL_MODE      = 0b00001000;
        const BREAK             = 0b00010000;
        const BREAK2            = 0b00100000;
        const OVERFLOW          = 0b01000000;
        const NEGATIVE          = 0b10000000;
    }
}

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: CpuFlags,
    pub program_counter: u16,
    pub stack_pointer: u8,
    pub memory: [u8; 0xFFFF],
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            stack_pointer: STACK_RESET,
            status: CpuFlags::from_bits_truncate(0b00100100),
            program_counter: 0,
            memory: [0; 0xFFFF],
        }
    }

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | lo
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a = value;
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_x = value;
        self.update_zero_and_negative_falgs(self.register_x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_y = value;
        self.update_zero_and_negative_falgs(self.register_y);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_y);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_falgs(self.register_x);
    }

    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_falgs(self.register_y);
    }

    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr).wrapping_add(1);
        self.mem_write(addr, value);
        self.update_zero_and_negative_falgs(value);
    }

    fn inx(&mut self) {
        self.update_zero_and_negative_falgs(self.register_x.wrapping_add(1));
    }

    fn iny(&mut self) {
        self.update_zero_and_negative_falgs(self.register_y.wrapping_add(1));
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.register_a &= self.mem_read(addr);
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.register_a |= self.mem_read(addr);
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.register_a ^= self.mem_read(addr);
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.register_a = self.add_to_register_a(self.mem_read(addr));
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let neg_value = !self.mem_read(addr) + 1;
        self.register_a = self.add_to_register_a(neg_value);
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn asl_accumulator(&mut self) {
        let shifted = (self.register_a as u16) << 1;
        self.status.set(CpuFlags::CARRY, shifted > 0xff);
        self.register_a = shifted as u8;
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let shifted = (self.mem_read(addr) as u16) << 1;
        self.status.set(CpuFlags::CARRY, shifted > 0xff);
        self.mem_write(addr, shifted as u8);
        self.update_zero_and_negative_falgs(self.mem_read(addr));
    }

    fn lsr_accumulator(&mut self) {
        self.status.set(CpuFlags::CARRY, self.register_a & 1 == 1);
        self.register_a >>= 1;
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.status.set(CpuFlags::CARRY, value & 1 == 1);
        self.mem_write(addr, value >> 1);
        self.update_zero_and_negative_falgs(self.mem_read(addr));
    }

    fn rol_accumulator(&mut self) {
        let old_carry = self.status.contains(CpuFlags::CARRY);
        let mut shifted = (self.register_a as u16) << 1;
        self.status.set(CpuFlags::CARRY, shifted > 0xff);
        if old_carry {
            shifted |= 1;
        }
        self.register_a = shifted as u8;
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn rol(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let old_carry = self.status.contains(CpuFlags::CARRY);
        let mut shifted = (self.register_a as u16) << 1;
        self.status.set(CpuFlags::CARRY, shifted > 0xff);
        if old_carry {
            shifted |= 1;
        }
        self.mem_write(addr, shifted as u8);
        self.update_zero_and_negative_falgs(self.mem_read(addr));
    }

    fn ror_accumulator(&mut self) {
        let old_carry = self.status.contains(CpuFlags::CARRY);
        self.status.set(CpuFlags::CARRY, self.register_a & 1 == 1);
        self.register_a >>= 1;
        if old_carry {
            self.register_a |= 0b1000_0000;
        }
        self.update_zero_and_negative_falgs(self.register_a);
    }

    fn ror(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let old_carry = self.status.contains(CpuFlags::CARRY);
        let mut value = self.mem_read(addr);
        self.status.set(CpuFlags::CARRY, value & 1 == 1);
        if old_carry {
            value |= 0b1000_0000;
        }
        self.mem_write(addr, value);
        self.update_zero_and_negative_falgs(self.mem_read(addr));
    }

    fn branch(&mut self, condition: bool) {
        if !condition {
            return;
        }
        let offset = self.mem_read(self.program_counter);
        self.program_counter = self
            .program_counter
            .wrapping_add(1)
            .wrapping_add(offset as u16);
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.status.set(CpuFlags::OVERFLOW, value & 0b0100_0000 != 0);
        self.status.set(CpuFlags::NEGATIVE, value & 0b1000_0000 != 0);
        self.status.set(CpuFlags::ZERO, value & self.register_a == 0);
    }

    fn add_to_register_a(&mut self, data: u8) -> u8 {
        let sum = self.register_a as u16
            + data as u16
            + if self.status.contains(CpuFlags::CARRY) {
                1
            } else {
                0
            };
        let carry = sum > 0xff;
        self.status.set(CpuFlags::CARRY, carry);
        let result = sum as u8;
        let is_same_sign = !(self.register_a ^ data) >> 7 & 1 == 1;
        let is_wrong_result = (sum >> 7 & 1 == 1) ^ carry;
        let overflow = is_same_sign && is_wrong_result;
        self.status.set(CpuFlags::OVERFLOW, overflow);
        result
    }

    fn update_zero_and_negative_falgs(&mut self, result: u8) {
        self.status.set(CpuFlags::ZERO, result == 0);
        self.status.set(CpuFlags::NEGATIVE, result & 0b1000_0000 != 0);
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            let opcode = opcodes
                .get(&code)
                .expect(&format!("OpCode {:x} is not recognized", code));

            match code {
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => self.adc(&opcode.mode),
                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => self.sbc(&opcode.mode),
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => self.lda(&opcode.mode),
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode),
                0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode),
                0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 => self.eor(&opcode.mode),
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe => self.ldx(&opcode.mode),
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => self.ldy(&opcode.mode),
                0x06 | 0x16 | 0x0e | 0x1e => self.asl(&opcode.mode),
                0x46 | 0x56 | 0x4e | 0x5e => self.lsr(&opcode.mode),
                0x26 | 0x36 | 0x2e | 0x3e => self.rol(&opcode.mode),
                0x66 | 0x76 | 0x6e | 0x7e => self.ror(&opcode.mode),
                0xe6 | 0xf6 | 0xee | 0xfe => self.inc(&opcode.mode),
                0x86 | 0x96 | 0x8e => self.stx(&opcode.mode),
                0x84 | 0x94 | 0x8c => self.sty(&opcode.mode),
                0x24 | 0x2c => self.bit(&opcode.mode),
                0x0a => self.asl_accumulator(),
                0x4a => self.lsr_accumulator(),
                0x2a => self.rol_accumulator(),
                0x6a => self.ror_accumulator(),
                0xe8 => self.inx(),
                0xc8 => self.iny(),
                0xaa => self.tax(),
                0x8a => self.txa(),
                0xa8 => self.tay(),
                0x98 => self.tya(),
                0xb0 => self.branch(self.status.contains(CpuFlags::CARRY)),
                0x90 => self.branch(!self.status.contains(CpuFlags::CARRY)),
                0xf0 => self.branch(self.status.contains(CpuFlags::ZERO)),
                0xd0 => self.branch(!self.status.contains(CpuFlags::ZERO)),
                0x30 => self.branch(self.status.contains(CpuFlags::NEGATIVE)),
                0x10 => self.branch(!self.status.contains(CpuFlags::NEGATIVE)),
                0x70 => self.branch(self.status.contains(CpuFlags::OVERFLOW)),
                0x50 => self.branch(!self.status.contains(CpuFlags::OVERFLOW)),
                0xea => { /* nop */ }
                0x00 => return,
                _ => todo!(),
            }

            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = CpuFlags::from_bits_truncate(0b00100100);
        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);
                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);
                let ptr: u8 = (base as u8).wrapping_add(self.register_y);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lda_from_memory_zeropage() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_from_memory_zeropage_x() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x15, 0x55);
        cpu.load_and_run(vec![0xa2, 0x05, 0xb5, 0x10, 0x00]);
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_from_memory_absolute() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x1000, 0x55);
        cpu.load_and_run(vec![0xad, 0x00, 0x10, 0x00]);
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_from_memory_indirect_x() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x1000, 0x55);
        cpu.mem_write_u16(0x15, 0x1000);
        cpu.load_and_run(vec![0xa2, 0x05, 0xa1, 0x10, 0x00]);
        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_adc_zeropage() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x01);
        cpu.load_and_run(vec![0xa9, 0x01, 0x65, 0x10, 0x00]);
        assert_eq!(cpu.register_a, 0x02);
    }

    #[test]
    fn test_adc_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x80, 0x69, 0x80, 0x00]);
        assert_eq!(cpu.register_a, 0x00);
        assert!(cpu.status.contains(CpuFlags::CARRY));
        assert!(cpu.status.contains(CpuFlags::ZERO));
        assert!(cpu.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
        cpu.load_and_run(vec![0xa9, 0x7f, 0x69, 0x7f, 0x00]);
        assert_eq!(cpu.register_a, 0xfe);
        assert!(!cpu.status.contains(CpuFlags::CARRY));
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(cpu.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
    }

    #[test]
    fn test_asl() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0x0a, 0x00]);
        assert_eq!(cpu.register_a, 0xfe);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_lsr() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0x4a, 0x00]);
        assert_eq!(cpu.register_a, 0x7f);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_rol() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0x0a, 0x2a, 0x00]);
        assert_eq!(cpu.register_a, 0b1111_1101);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_ror() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0x4a, 0x6a, 0x00]);
        assert_eq!(cpu.register_a, 0b1011_1111);
        assert!(cpu.status.contains(CpuFlags::CARRY));
    }

    #[test]
    fn test_bit() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0xff);
        cpu.load_and_run(vec![0xa9, 0xff, 0x24, 0x10, 0x00]);
        assert!(!cpu.status.contains(CpuFlags::ZERO));
        assert!(cpu.status.contains(CpuFlags::OVERFLOW));
        assert!(cpu.status.contains(CpuFlags::NEGATIVE));
        cpu.mem_write(0x10, 0x00);
        cpu.load_and_run(vec![0xa9, 0xff, 0x24, 0x10, 0x00]);
        assert!(cpu.status.contains(CpuFlags::ZERO));
        assert!(!cpu.status.contains(CpuFlags::OVERFLOW));
        assert!(!cpu.status.contains(CpuFlags::NEGATIVE));
    }
}
