#[derive(Copy, Clone)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    OneScreenLower,
    OneScreenUpper,
    FourScreen,
}

pub fn mirroring_mode_name(mode: Mirroring) -> &'static str {
    match mode {
        Mirroring::Horizontal => "Horizontal",
        Mirroring::Vertical => "Vertical",
        Mirroring::OneScreenLower => "OneScreenLower",
        Mirroring::OneScreenUpper => "OneScreenUpper",
        Mirroring::FourScreen => "FourScreen",
        _ => "Invalid"
    }
}

pub trait Mapper: Send {
    fn read_cpu(&mut self, address: u16) -> Option<u8>;
    fn write_cpu(&mut self, address: u16, data: u8);
    fn read_ppu(&mut self, address: u16) -> Option<u8>;
    fn write_ppu(&mut self, address: u16, data: u8);
    fn debug_read_cpu(&mut self, address: u16) -> Option<u8> {return self.read_cpu(address);}
    fn debug_read_ppu(&mut self, address: u16) -> Option<u8> {return self.read_ppu(address);}
    fn print_debug_status(&self) {}
    fn mirroring(&self) -> Mirroring;
    fn has_sram(&self) -> bool {return false;}
    fn get_sram(&self) -> Vec<u8> {return vec![0u8; 0];}
    fn load_sram(&mut self, _: Vec<u8>) {}
    fn irq_flag(&self) -> bool {return false;}
}
