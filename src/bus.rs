use crate::memory::Memory;

pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);

    fn poll_nmi(&self) -> bool {
        false
    }
    fn poll_irq(&self) -> bool {
        false
    }

    fn acknowledge_nmi(&mut self) {}
}

impl Bus for Memory {
    fn read(&mut self, addr: u16) -> u8 {
        self.0[addr as usize]
    }

    fn write(&mut self, addr: u16, val: u8) {
        self.0[addr as usize] = val;
    }
}
