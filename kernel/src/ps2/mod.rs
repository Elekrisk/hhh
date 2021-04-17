pub mod keyboard;

use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;

use crate::idt;
use crate::pic;

pub struct Ps2Driver {}

static PS2DRIVER: Mutex<Ps2Driver> = Mutex::new(Ps2Driver::new());

impl Ps2Driver {
    pub const fn new() -> Self {
        Self {}
    }

    pub unsafe fn initialize(&mut self) {
        // Disable ports
        self.write_command(0xAD);
        self.write_command(0xA7);

        // Flush output buffer
        if self.read_status().output_buffer_full() {
            self.read_data();
        }

        // Set config
        self.write_command(0x20); // Read config
        let mut config: Config = self.read_data().into();
        config.first_port_interrupt = false;
        config.second_port_interrupt = false;
        config.first_port_translation = false;
        self.write_command(0x60); // Write config
        self.write_data(config.into());

        // Perform self test
        self.write_command(0xAA); // Self test
        let response = self.read_data();
        if response != 0x55 {
            println!(
                "PS/2 controller failed self test: response was {:2x}",
                response
            );
        }
        println!("PS/2 controller passed self test");

        // Test port 1
        self.write_command(0xAB); // Test port 1
        let response = self.read_data();
        if response != 0x00 {
            println!("PS/2 port 1 failed self test: response was {:2x}", response);
        }
        println!("PS/2 port 1 passed self test");

        // Register ISR for IRQ1
        idt::register_isr(0x20 + 1, irq1);

        // Make sure IRQ1 is enabled
        pic::enable_irq(1);

        // Enable interrupts from the first port
        self.write_command(0x20);
        let mut config: Config = self.read_data().into();
        config.first_port_interrupt = true;
        self.write_command(0x60);
        self.write_data(config.into());
        println!("new config: {:08b}", <Config as Into<u8>>::into(config));

        // Enable first port
        self.write_command(0xAE);

        // self.monitor_status();
    }

    unsafe fn read_data(&self) -> u8 {
        while !self.read_status().output_buffer_full() {}
        let mut out;
        asm!("in al, 0x60", out("al") out);
        out
    }

    unsafe fn write_data(&mut self, data: u8) {
        while self.read_status().input_buffer_full() {}
        asm!("out 0x60, al", in("al") data, options(nostack));
    }

    unsafe fn read_status(&self) -> Status {
        let mut out;
        asm!("in al, 0x64", out("al") out);
        Status::new(out)
    }

    unsafe fn write_command(&mut self, command: u8) {
        while self.read_status().input_buffer_full() {}
        asm!("out 0x64, al", in("al") command, options(nostack));
    }

    unsafe fn monitor_status(&mut self) -> ! {
        let mut last_status = self.read_status().data;
        println!("Initial status: {:08b}", last_status);
        loop {
            let status = self.read_status().data;
            if last_status != status {
                last_status = status;
                println!("Status changed: {:08b}", status);
            }
        }
    }
}

extern "x86-interrupt" fn irq1(stack_frame: InterruptStackFrame) {
    let message = unsafe { PS2DRIVER.lock().read_data() };

    keyboard::handle_message(message);

    unsafe { pic::send_eoi(1) };
}

#[derive(Clone, Copy)]
struct Config {
    first_port_interrupt: bool,
    second_port_interrupt: bool,
    first_port_clock: bool,
    second_port_clock: bool,
    first_port_translation: bool,
    _data: u8,
}

impl From<u8> for Config {
    fn from(val: u8) -> Self {
        Self {
            first_port_interrupt: val & 1 << 0 > 0,
            second_port_interrupt: val & 1 << 1 > 0,
            first_port_clock: val & 1 << 4 == 0,
            second_port_clock: val & 1 << 5 == 0,
            first_port_translation: val & 1 << 6 > 0,
            _data: val,
        }
    }
}

impl From<Config> for u8 {
    fn from(config: Config) -> Self {
        let mut val = config._data;
        val &= 0b10001100; // Clear the bits we're about to set
        if config.first_port_interrupt {
            val |= 1 << 0;
        }
        if config.second_port_interrupt {
            val |= 1 << 1;
        }
        // Clock enabled bits are inverted (1 == disabled)
        if !config.first_port_clock {
            val |= 1 << 4;
        }
        if !config.second_port_clock {
            val |= 1 << 5;
        }
        if config.first_port_translation {
            val |= 1 << 6;
        }
        val
    }
}

struct Status {
    data: u8,
}

impl Status {
    fn new(data: u8) -> Self {
        Self { data }
    }

    fn output_buffer_full(&self) -> bool {
        self.data & 1 << 0 > 0
    }

    fn input_buffer_full(&self) -> bool {
        self.data & 1 << 1 > 0
    }

    fn system_flag(&self) -> bool {
        self.data & 1 << 2 > 0
    }

    fn expecting_command(&self) -> bool {
        self.data & 1 << 3 > 0
    }

    fn bit_4(&self) -> bool {
        self.data & 1 << 3 > 0
    }

    fn bit_5(&self) -> bool {
        self.data & 1 << 4 > 0
    }

    fn time_out_error(&self) -> bool {
        self.data & 1 << 5 > 0
    }

    fn parity_error(&self) -> bool {
        self.data & 1 << 6 > 0
    }
}
