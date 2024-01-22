mod pwm_streamer;
use std::time::Duration;

pub use pwm_streamer::*;

use readformat::{readf, readf1};
pub use serialport;
use serialport::{Error, SerialPort};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PicoGPIOVersion {
    V1_0,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PinValueRead {
    Floating(Option<bool>),
    Analog(u32),
    Digital(bool),
    PWM(u32),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PinValueWrite {
    Floating,
    Digital(bool),
    PWM(u32),
}

impl PinValueRead {
    pub fn matches(&self, written: &PinValueWrite) -> bool {
        match (self, written) {
            (PinValueRead::Floating(_), PinValueWrite::Floating) => true,
            (PinValueRead::Digital(a), PinValueWrite::Digital(b)) if a == b => true,
            (PinValueRead::PWM(a), PinValueWrite::PWM(b)) if a == b => true,
            _ => false,
        }
    }

    pub fn matches_in(&self, asked: &PinInput) -> bool {
        matches!(
            (self, asked),
            (PinValueRead::Floating(_), PinInput::Floating)
                | (PinValueRead::Analog(_), PinInput::Analog)
                | (PinValueRead::Digital(_), PinInput::PDown)
                | (PinValueRead::Digital(_), PinInput::PUp)
        )
    }
}

/*
impl From<PinValueRead> for PinValueWrite {
    fn from(value: PinValueRead) -> Self {
        match value {
            PinValueRead::Floating(_) => PinValueWrite::Floating,
            PinValueRead::Analog(_) => PinValueWrite::Floating,
            PinValueRead::Digital(b) => PinValueWrite::Digital(b),
            PinValueRead::PWM(v) => PinValueWrite::PWM(v),
        }
    }
}
*/

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PinInput {
    Analog,
    Floating,
    PDown,
    PUp,
}

struct Params<PVType, const PINS: usize> {
    pins: [PVType; PINS],
    pwmfreq: u32,
    pwmres: u8,
    inares: u8,
    streaming: bool,
}

pub struct PicoGPIO<Port: SerialPort, const PINS: usize = 256> {
    port: Port,
    version: PicoGPIOVersion,
    // Intended values, set immediately
    intended: Params<PinValueWrite, PINS>,
    // Actual values, set when receiving a response
    actual: Params<PinValueRead, PINS>,
    blocking: bool,
}

impl<Port: SerialPort, const PINS: usize> PicoGPIO<Port, PINS> {
    pub fn new(mut serial_port: Port) -> Result<Self, Error> {
        serial_port.set_timeout(Duration::from_millis(500))?;
        serial_port.write_all("\r\n".as_bytes())?;
        Ok(Self {
            port: serial_port,
            version: PicoGPIOVersion::Unknown,
            intended: Params {
                pins: [PinValueWrite::Floating; PINS],
                pwmfreq: 0,
                pwmres: 8,
                inares: 10,
                streaming: false,
            },
            actual: Params {
                pins: [PinValueRead::Floating(None); PINS],
                pwmfreq: 0,
                pwmres: 8,
                inares: 10,
                streaming: false,
            },
            blocking: true,
        })
    }

    pub fn poll(&mut self, mut min_lines: usize) -> Result<(), Error> {
        loop {
            if self.port.bytes_to_read()? == 0 && min_lines == 0 {
                break;
            }
            let mut buf = [0u8; 1];
            self.port.read_exact(&mut buf)?;
            let mut line = vec![buf[0]];
            loop {
                let mut buf = [0u8; 1024];
                let amt = self.port.read(&mut buf)?;
                line.append(&mut buf[..amt].to_vec());
                if *line.last().unwrap() as char == '\n' {
                    break;
                }
            }
            for line in String::from_utf8(line).unwrap().split('\n') {
                self.parse_line(line.trim())?;
                min_lines = min_lines.saturating_sub(1);
            }
        }
        Ok(())
    }

    fn parse_line(&mut self, line: &str) -> Result<(), Error> {
        if line == "!OK" {
            // pass
        } else if let Some(v) = readf1("+PICO_GPIO {}", line) {
            self.version = match v.as_str() {
                "V1.0" => PicoGPIOVersion::V1_0,
                _ => PicoGPIOVersion::Unknown,
            };
        } else if let Some(err) = readf1("!ERROR:{}", line) {
            panic!("PicoGPIO version mismatch: ERROR {err}.");
        } else if let Some(freq) = readf1("!PWMFREQ:{}", line) {
            self.actual.pwmfreq = freq.parse().unwrap();
        } else if let Some(res) = readf1("!PWMRES:{}", line) {
            self.actual.pwmres = res.parse().unwrap();
        } else if let Some(res) = readf1("!INARES:{}", line) {
            self.actual.inares = res.parse().unwrap();
        } else if line == "!STREAMING" {
            self.actual.streaming = true;
        } else if let Some([pin, val]) = readf("~{}={}", line).as_deref() {
            self.actual.pins[pin.parse::<usize>().expect("invalid data from PicoGPIO!")] =
                PinValueRead::Floating(Some(
                    val.parse::<u8>().expect("invalid data from PicoGPIO!") != 0,
                ))
        } else if let Some([pin, val]) = readf("/{}={}", line).as_deref() {
            self.actual.pins[pin.parse::<usize>().expect("invalid data from PicoGPIO!")] =
                PinValueRead::Analog(val.parse::<u32>().expect("invalid data from PicoGPIO!"))
        } else if let Some([pin, val]) = readf("#{}={}", line).as_deref() {
            self.actual.pins[pin.parse::<usize>().expect("invalid data from PicoGPIO!")] =
                PinValueRead::PWM(val.parse::<u32>().expect("invalid data from PicoGPIO!"))
        } else if let Some([pin, val]) = readf("{}={}", line).as_deref() {
            self.actual.pins[pin.parse::<usize>().expect("invalid data from PicoGPIO!")] =
                PinValueRead::Digital(val.parse::<u8>().expect("invalid data from PicoGPIO!") != 0)
        }
        Ok(())
    }

    pub fn set_manual(
        &mut self,
        pin: usize,
        value: PinValueWrite,
        block: bool,
    ) -> Result<(), Error> {
        self.intended.pins[pin] = value;
        match value {
            PinValueWrite::Floating => {
                self.port.write_all(format!("float {pin}\r\n").as_bytes())?
            }
            PinValueWrite::Digital(val) => self
                .port
                .write_all(format!("out {pin}={}\r\n", if val { 1 } else { 0 }).as_bytes())?,
            PinValueWrite::PWM(val) => self
                .port
                .write_all(format!("pwm {pin}={val}\r\n").as_bytes())?,
        }
        self.poll(0)?;
        if block {
            while !self.actual.pins[pin].matches(&value) {
                self.poll(1)?;
            }
        }
        Ok(())
    }

    pub fn get_manual(
        &mut self,
        pin: usize,
        kind: PinInput,
        cached: bool,
        block: bool,
    ) -> Result<PinValueRead, Error> {
        if !cached {
            self.intended.pins[pin] = PinValueWrite::Floating;
            self.poll(0)?;
            match kind {
                PinInput::Floating => self.port.write_all(format!("float {pin}\r\n").as_bytes())?,
                PinInput::Analog => self.port.write_all(format!("ina {pin}\r\n").as_bytes())?,
                PinInput::PDown => self.port.write_all(format!("in {pin}\r\n").as_bytes())?,
                PinInput::PUp => self.port.write_all(format!("in^ {pin}\r\n").as_bytes())?,
            }
            if block {
                self.poll(1)?;
            }
            while !self.actual.pins[pin].matches_in(&kind) {
                self.poll(1)?;
            }
        }

        Ok(self.actual.pins[pin])
    }

    pub fn float(&mut self, pin: usize) -> Result<(), Error> {
        self.set_manual(pin, PinValueWrite::Floating, self.blocking)
    }

    pub fn d_out(&mut self, pin: usize, val: bool) -> Result<(), Error> {
        self.set_manual(pin, PinValueWrite::Digital(val), self.blocking)
    }

    pub fn pwm_out(&mut self, pin: usize, val: u32) -> Result<(), Error> {
        self.set_manual(pin, PinValueWrite::PWM(val), self.blocking)
    }

    pub fn float_in(&mut self, pin: usize) -> Result<bool, Error> {
        self.get_manual(pin, PinInput::Floating, false, self.blocking)
            .map(|x| match x {
                PinValueRead::Floating(Some(x)) => x,
                _ => unreachable!(),
            })
    }

    pub fn pulldn_in(&mut self, pin: usize) -> Result<bool, Error> {
        self.get_manual(pin, PinInput::PDown, false, self.blocking)
            .map(|x| match x {
                PinValueRead::Digital(x) => x,
                _ => unreachable!(),
            })
    }

    pub fn pullup_in(&mut self, pin: usize) -> Result<bool, Error> {
        self.get_manual(pin, PinInput::PUp, false, self.blocking)
            .map(|x| match x {
                PinValueRead::Digital(x) => x,
                _ => unreachable!(),
            })
    }

    pub fn init_pwm(&mut self, freq: u32, res: u8, block: bool) -> Result<(), Error> {
        self.intended.pwmres = res;
        self.intended.pwmfreq = freq;
        self.port
            .write_all(format!("pwmres {res}\r\npwmfreq {freq}\r\n").as_bytes())?;
        self.poll(0)?;
        if block {
            while self.actual.pwmfreq != freq || self.actual.pwmres != res {
                self.poll(1)?;
            }
        }
        Ok(())
    }

    pub fn pwmstream(mut self, pin: usize) -> Result<PwmStreamer<Port, PINS>, (Self, Error)> {
        if let Err(e) = self
            .init_pwm(self.intended.pwmfreq, 8, true)
            .and_then(|()| {
                self.port
                    .write_all(format!("pwmstream {pin}").as_bytes())
                    .map_err(|x| x.into())
            })
        {
            return Err((self, e));
        }
        Ok(PwmStreamer::new(self, PwmStreamMode::PWM))
    }

    pub fn audiostream(mut self, pin: usize) -> Result<PwmStreamer<Port, PINS>, (Self, Error)> {
        if let Err(e) = self
            .init_pwm(self.intended.pwmfreq, 8, true)
            .and_then(|()| {
                self.port
                    .write_all(format!("audiostream {pin}").as_bytes())
                    .map_err(|x| x.into())
            })
        {
            return Err((self, e));
        }
        Ok(PwmStreamer::new(self, PwmStreamMode::Audio))
    }
}
