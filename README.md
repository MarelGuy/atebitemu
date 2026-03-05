# AteBitEmu

This is a MOS 6502 emulator written in rust.

## Installation

You can find this program on cargo, future releases may be available on the AUR or launchpad.

```
cargo install atebitemu
```

if you prefer a binary, you can find it in the [releases](https://github.com/MarelGuy/atebitemu/releases) page


## Usage

**atebitemu** [OPTIONS] <PROGRAM_PATH>

**Arguments**:
  <PROGRAM_PATH>  Path to the binary program to load

**Options:**
  **-u,** **--unlimited** Run the emulator without a clock speed limit  
  **-m,** **--mhz** <MHZ> CPU clock speed in MHz [default: 1]  
  **-t**, **--trace** Print CPU state after every instruction (Trace mode)  
  **-s**, **--start-pc** <START_PC> Override the starting Program Counter (PC) address (in hex)  
  **-l**, **--load-addr** <LOAD_ADDR> Address to load the program into (in hex) [default: 0x8000]  
  **-r**, **--reset-vector** <RESET_VECTOR> Address to set the reset vector to (defaults to `load_addr`)  
  **-h**, **--help** Print help  
  **-V**, **--version** Print version

## Tests 

Tests are not included in the source code, but you can download/assemble [Klaus Dormann's 6502 tests](https://github.com/Klaus2m5/6502_65C02_functional_tests/tree/master), or write your own.

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPLv3). Any modifications or redistributed versions (including those used over a network) must remain open source under this same license.
