use serialport::{Error, SerialPort};

use crate::PicoGPIO;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PwmStreamMode {
    PWM,
    Audio,
}

pub struct PwmStreamer<Port: SerialPort, const PINS: usize> {
    original: PicoGPIO<Port, PINS>,
    mode: PwmStreamMode,
}

impl<Port: SerialPort, const PINS: usize> PwmStreamer<Port, PINS> {
    pub(crate) fn new(original: PicoGPIO<Port, PINS>, mode: PwmStreamMode) -> Self {
        Self { original, mode }
    }

    pub fn get_mode(&self) -> PwmStreamMode {
        self.mode
    }

    pub fn submit_data(&mut self, data: Vec<u8>) -> Result<(), Error> {
        self.original.port.write_all(&data)?;
        Ok(())
    }
}
