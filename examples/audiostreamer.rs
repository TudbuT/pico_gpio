use std::io::{stdin, Read};

use pico_gpio::PicoGPIO;

fn main() -> Result<(), serialport::Error> {
    let port = serialport::new("/dev/ttyACM0", 2000000)
        .open_native()
        .unwrap();
    let mut gpio: PicoGPIO<_> = PicoGPIO::new(port).unwrap();
    println!("Init...");
    gpio.init_pwm(115200, 8, true)?;
    println!("Start...");
    let mut stream = gpio.audiostream(0).ok().unwrap();
    println!("Streaming.");

    let mut buf = [0u8; 1024];
    let mut stdin = stdin().lock();
    loop {
        stdin.read_exact(&mut buf).unwrap();
        stream.submit_data(&buf)?;
    }
}
