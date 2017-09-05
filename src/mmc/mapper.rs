use cartridge::NesHeader;

pub trait Mapper {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, data: u8);
    fn print_debug_status(&self) {}
}
