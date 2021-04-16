
pub struct Ps2Driver {

}

impl Ps2Driver {
    pub fn new() -> Self {
        Self {

        }
    }

    pub unsafe fn initialize(&mut self) {
        self.write_command(0x20);
        let config_byte = self.read_data();
        println!("Config byte: {:02x}", config_byte);
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
}

struct Status {
    data: u8
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
