pub struct Memory(pub Vec<u8>);

impl Memory {
    pub fn new(rom_data: &[u8]) -> Self {
        let mut mem = vec![0u8; 65536];

        if rom_data.len() == 65536 {
            mem.copy_from_slice(rom_data);
        } else {
            let length_to_copy = (0x8000 + rom_data.len()).min(0x10000) - 0x8000;
            mem[0x8000..0x8000 + length_to_copy].copy_from_slice(&rom_data[..length_to_copy]);

            if (0x8000 + rom_data.len()) <= 0xFFFC {
                mem[0xFFFC] = 0x00;
                mem[0xFFFD] = 0x80;
            }
        }

        Self(mem)
    }
}
