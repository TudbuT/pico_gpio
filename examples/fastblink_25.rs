use std::time::{Duration, SystemTime};

use pico_gpio::PicoGPIO;

fn main() -> Result<(), serialport::Error> {
    let port = serialport::new("/dev/ttyACM0", 2000000)
        .open_native()
        .unwrap();
    let mut gpio: PicoGPIO<_> = PicoGPIO::new(port).unwrap();

    let t = SystemTime::now();
    for _ in 0..1000 {
        gpio.out_d(25, true)?;
        gpio.out_d(25, false)?;
    }
    println!("{}ms for 2000 writes", t.elapsed().unwrap().as_millis());
    Ok(())
}
