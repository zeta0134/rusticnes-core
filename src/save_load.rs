use std::{convert::TryInto};

pub(crate) fn save_usize(buff: &mut Vec<u8>, data: usize) {
    buff.extend(&data.to_le_bytes());
}
pub(crate) fn load_usize(buff: &mut Vec<u8>) -> usize {
    usize::from_le_bytes(buff.split_off(buff.len() - std::mem::size_of::<usize>()).try_into().unwrap())
}

pub(crate) fn save_u16(buff: &mut Vec<u8>, data: u16) {
    buff.extend(data.to_le_bytes());
}
pub(crate) fn load_u16(buff: &mut Vec<u8>) -> u16 {
    u16::from_le_bytes(buff.split_off(buff.len() - std::mem::size_of::<u16>()).try_into().unwrap())
}

pub(crate) fn save_u32(buff: &mut Vec<u8>, data: u32) {
    buff.extend(data.to_le_bytes());
}
pub(crate) fn load_u32(buff: &mut Vec<u8>) -> u32 {
    u32::from_le_bytes(buff.split_off(buff.len() - std::mem::size_of::<u32>()).try_into().unwrap())
}

pub(crate) fn save_u64(buff: &mut Vec<u8>, data: u64) {
    buff.extend(data.to_le_bytes());
}
pub(crate) fn load_u64(buff: &mut Vec<u8>) -> u64 {
    u64::from_le_bytes(buff.split_off(buff.len() - std::mem::size_of::<u64>()).try_into().unwrap())
}
pub(crate) fn save_bool(buff: &mut Vec<u8>, data: bool) {
    buff.push(data as u8);
}
pub(crate) fn load_bool(buff: &mut Vec<u8>) -> bool {
    buff.pop().unwrap() != 0
}

pub(crate) fn save_vec(buff: &mut Vec<u8>, data: &Vec<u8>) {
    buff.extend(data);
}
pub(crate) fn load_vec(buff: &mut Vec<u8>, len: usize) -> Vec<u8> {
    buff.split_off(buff.len() - len)
}