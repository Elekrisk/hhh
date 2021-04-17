unsafe fn write_master_command(command: u8) {
    asm!("out 0x20, al", in("al") command, options(nostack));
}

unsafe fn write_master_data(data: u8) {
    asm!("out 0x21, al", in("al") data, options(nostack));
}

unsafe fn write_slave_command(command: u8) {
    asm!("out 0xA0, al", in("al") command, options(nostack));
}

unsafe fn write_slave_data(data: u8) {
    asm!("out 0xA1, al", in("al") data, options(nostack));
}

unsafe fn read_master_data() -> u8 {
    let mut out;
    asm!("in al, 0x21", out("al") out, options(nostack));
    out
}

unsafe fn read_slave_data() -> u8 {
    let mut out;
    asm!("in al, 0xA1", out("al") out, options(nostack));
    out
}

unsafe fn read_master_response() -> u8 {
    let mut out;
    asm!("in al, 0x20", out("al") out, options(nostack));
    out
}

unsafe fn read_slave_response() -> u8 {
    let mut out;
    asm!("in al, 0xA0", out("al") out, options(nostack));
    out
}

pub unsafe fn disable() {
    write_master_data(0xFF);
    write_slave_data(0xFF);
}

pub unsafe fn initialize() {
    let old_master_mask = read_master_data();
    let old_slave_mask = read_slave_data();

    write_master_command(0x10 | 0x01);
    wait_io_cycle();
    write_slave_command(0x10 | 0x01);
    wait_io_cycle();
    write_master_data(0x20);
    wait_io_cycle();
    write_slave_data(0x28);
    wait_io_cycle();
    write_master_data(4);
    wait_io_cycle();
    write_slave_data(2);
    wait_io_cycle();
    write_master_data(1);
    wait_io_cycle();
    write_slave_data(1);
    wait_io_cycle();

    write_master_data(old_master_mask);
    write_slave_data(old_slave_mask);

    println!(
        "pic mask: {:016b}",
        (old_slave_mask as u16) << 8 | old_master_mask as u16
    );
}

/// Waits one IO cycle by writing to a unused port.
unsafe fn wait_io_cycle() {
    asm!("out 0x80, al", in("al") 0u8, options(nostack))
}

pub unsafe fn enable_irq(irq: u8) {
    assert!(irq < 16);
    if irq < 8 {
        let mut mask = read_master_data();
        mask &= !(1 << irq);
        write_master_data(mask);
    } else {
        let mut mask = read_slave_data();
        mask &= !(1 << irq - 8);
        write_slave_data(mask);
    }
    let a = read_master_data() as u16;
    let b = read_master_data() as u16;
    println!("New mask: {:016b}", a << 8 | b);
}

pub unsafe fn disable_irq(irq: u8) {
    assert!(irq < 16);
    if irq < 8 {
        let mut mask = read_master_data();
        mask |= 1 << irq;
        write_master_data(mask);
    } else {
        let mut mask = read_slave_data();
        mask |= 1 << irq - 8;
        write_slave_data(mask);
    }
    let a = read_master_data() as u16;
    let b = read_master_data() as u16;
    println!("New mask: {:016b}", a << 8 | b);
}

pub unsafe fn send_eoi(irq: u8) {
    if irq < 8 {
        write_master_command(0x20);
    } else {
        write_master_command(0x20);
        write_slave_command(0x20);
    }
}

pub unsafe fn get_isr() -> u16 {
    write_master_command(0x0B);
    let a = read_master_response() as u16;
    write_slave_command(0x0B);
    let b = read_slave_response() as u16;
    b << 8 | a
}

pub unsafe fn get_irr() -> u16 {
    write_master_command(0x0A);
    let a = read_master_response() as u16;
    write_slave_command(0x0A);
    let b = read_slave_response() as u16;
    b << 8 | a
}
