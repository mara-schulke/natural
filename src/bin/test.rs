use pgpt::driver::Driver;

fn main() {
    std::thread::spawn(|| {
        let mut driver = Driver::boot();

        loop {
            driver.push();
        }
    });

    let handle = Driver::attach();

    dbg!(handle.prompt("Hi model!"));
}
