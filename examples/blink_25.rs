use std::{thread::sleep, time::Duration};

use pico_gpio::PicoGPIO;

fn main() -> Result<(), serialport::Error> {
    let port = serialport::new("/dev/ttyACM0", 2000000)
        .open_native()
        .unwrap();
    let mut gpio: PicoGPIO<_> = PicoGPIO::new(port).unwrap();

    loop {
        gpio.d_out(25, true)?;
        sleep(Duration::from_millis(500));
        gpio.d_out(25, false)?;
        sleep(Duration::from_millis(500));
    }
}
