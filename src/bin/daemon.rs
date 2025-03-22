use std::{io::Read, os::unix::net::UnixListener};

use daemonize::Daemonize;
use pgpt::driver::Driver;

fn main() -> std::io::Result<()> {
    //let daemon = Daemonize::new().pid_file("/tmp/inference-daemon.pid");

    //daemon.start().unwrap();

    let socket_path = "/tmp/pgptid";

    let listener = UnixListener::bind(socket_path)?;

    std::thread::spawn(|| {
        let mut driver = Driver::boot();

        loop {
            driver.push();
        }
    });

    let handle = Driver::attach();

    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut pbuf = Vec::new();

        stream.read_to_end(&mut pbuf)?;

        let prompt: String = serde_json::from_slice(&pbuf)?;

        let answer = handle.prompt(prompt);

        serde_json::to_writer(&mut stream, &answer)?;
    }

    Ok(())
}
