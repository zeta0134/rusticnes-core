#[derive(Copy, Clone)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    OneScreenLower,
    OneScreenUpper,
    FourScreen,
}

pub trait Mapper {
    fn read_byte(&mut self, address: u16) -> Option<u8>;
    fn write_byte(&mut self, address: u16, data: u8);
    fn debug_read_byte(&mut self, address: u16) -> Option<u8> {return self.read_byte(address);}
    fn print_debug_status(&self) {}
    fn mirroring(&self) -> Mirroring;
    fn has_sram(&self) -> bool {return false;}
    fn get_sram(&self) -> Vec<u8> {return vec![0u8; 0];}
    fn load_sram(&mut self, _: Vec<u8>) {}
    fn irq_flag(&self) -> bool {return false;}
}
