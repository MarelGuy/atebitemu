### AteBitEmu

This is a MOS 6502 emulator using rust.

## Usage
**atebitemu** [OPTIONS] <PROGRAM_PATH>

**Arguments**:
  <PROGRAM_PATH>  Path to the binary program to load

**Options:**
  **-u,** **--unlimited** Run the emulator without a clock speed limit  
  **-m,** **--mhz** <MHZ> CPU clock speed in MHz [default: 1]  
  **-t**, **--trace** Print CPU state after every instruction (Trace mode)  
  **-s**, **--start-pc** <START_PC> Override the starting Program Counter (PC) address (in hex)  
  **-h**, **--help** Print help  
  **-V**, **--version** Print version  