use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_char;
use flipperzero_sys as sys;
use sys::c_string;

static CMD: u8 = 0xCD;

#[derive(Clone, Copy)]
pub enum TagSize {
    // must match location in menu
    TwoNine = 0,
    FourTwo = 1,
    SevenFive = 2,
}

impl TagSize {
    pub fn text(&self) -> *const c_char {
        match self {
            Self::TwoNine => c_string!("2.9\""),
            Self::FourTwo => c_string!("4.2\""),
            Self::SevenFive => c_string!("7.5\""),
        }
    }

    pub fn id(&self) -> u8 {
        match self {
            Self::TwoNine => 0x07,
            Self::FourTwo => 0x0A,
            Self::SevenFive => 0x0E,
        }
    }

    pub fn width(&self) -> usize {
        match self {
            Self::TwoNine => 128,
            Self::FourTwo => 400,
            Self::SevenFive => 800,
        }
    }

    pub fn height(&self) -> usize {
        match self {
            Self::TwoNine => 296,
            Self::FourTwo => 300,
            Self::SevenFive => 480,
        }
    }

    pub fn chunk_size(&self) -> u8 {
        match self {
            Self::TwoNine => 16,
            Self::FourTwo => 100,
            Self::SevenFive => 120,
        }
    }

    pub fn setup(&self) -> Vec<Vec<u8>> {
        vec![
            vec![CMD, 0x0D],
            vec![CMD, 0x00, self.id()],
            vec![CMD, 0x01],
            vec![CMD, 0x02],
            vec![CMD, 0x03],
            vec![CMD, 0x05],
            vec![CMD, 0x06],
            vec![CMD, 0x07, 0x00],
        ]
    }

    pub fn buffer(&self) -> (Vec<u8>, usize) {
        let chunk_size = self.chunk_size();
        let preamble = [CMD, 0x08, chunk_size];
        let mut buffer = vec![0u8; preamble.len() + chunk_size as usize];
        buffer[0..preamble.len()].copy_from_slice(&preamble);
        (buffer, preamble.len())
    }

    pub fn loops(&self) -> usize {
        (self.width() * self.height()) / (self.chunk_size() as usize * 8)
    }

    pub fn power_on(&self) -> Vec<u8> {
        vec![CMD, 0x18]
    }

    pub fn refresh(&self) -> Vec<u8> {
        vec![CMD, 0x09]
    }

    pub fn wait(&self) -> Vec<u8> {
        vec![CMD, 0x0A]
    }

    pub fn power_off(&self) -> Vec<u8> {
        vec![CMD, 0x04]
    }
}
