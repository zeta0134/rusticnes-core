struct Flags = {
    carry: bool,
    zero: bool,
    interrupt: bool,
    decimal: bool,
    overflow: bool,
    negative: bool,
}

struct Registers = {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    s: u8,
    flags: Flags,
};
