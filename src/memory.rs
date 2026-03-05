pub struct Memory(pub Vec<u8>);

impl Memory {
    pub fn new() -> Self {
        Self(vec![0u8; 65536])
    }

    pub fn load_at(&mut self, addr: u16, data: &[u8]) {
        let start = addr as usize;
        let end = (start + data.len()).min(65536);
        let len = end - start;

        self.0[start..end].copy_from_slice(&data[..len]);
    }

    pub fn write_u16(&mut self, addr: u16, val: u16) {
        self.0[addr as usize] = (val & 0xFF) as u8;
        self.0[(addr + 1) as usize] = (val >> 8) as u8;
    }
}
