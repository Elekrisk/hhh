use spin::Mutex;
use x86_64::{instructions::port::Port, structures::{idt::InterruptStackFrame, port::{PortRead, PortWrite}}};
use alloc::prelude::v1::*;

use crate::{idt, pic};

static PATA_DRIVER: Mutex<PataDriver> = Mutex::new(unsafe { PataDriver::default() });

struct PataDriver {
    default_bus: Bus,
}

extern "x86-interrupt" fn irq14(stack_frame: InterruptStackFrame) {
    println!("IRQ 14");
    unsafe { pic::send_eoi(14) };
}

impl PataDriver {
    const unsafe fn default() -> Self {
        Self {
            default_bus: Bus {
                io_base: 0x1F0,
                ctl_base: 0x3F6,
                master_drive: NewDiskInfo::Uninitialized,
                slave_drive: NewDiskInfo::Uninitialized,
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiskSelect {
    Master,
    Slave
}

pub unsafe fn init() {
    PATA_DRIVER.lock().default_bus.initialize().unwrap();
}

pub unsafe fn read_sectors(drive_select: DiskSelect, sector: u64, buffer: &mut [u8]) -> Result<(), String> {
    PATA_DRIVER.lock().default_bus.read_sectors(drive_select, sector, buffer)
}


pub struct Bus {
    io_base: u16,
    ctl_base: u16,
    master_drive: NewDiskInfo,
    slave_drive: NewDiskInfo
}

impl Bus {
    const unsafe fn new(io_base: u16, ctl_base: u16) -> Self {
        Self {
            io_base,
            ctl_base,
            master_drive: NewDiskInfo::Uninitialized,
            slave_drive: NewDiskInfo::Uninitialized
        }
    }

    unsafe fn write_ctl(&mut self, offset: u16, value: u8) {
        let mut port = Port::new(self.ctl_base + offset);
        port.write(value);
    }

    unsafe fn write_io<T: PortWrite>(&mut self, offset: u16, value: T) {
        let mut port = Port::new(self.io_base + offset);
        port.write(value);
    }

    unsafe fn read_ctl(&mut self, offset: u16) -> u8 {
        let mut port = Port::new(self.ctl_base + offset);
        port.read()
    }

    unsafe fn read_io<T: PortRead>(&mut self, offset: u16) -> T {
        let mut port = Port::new(self.io_base + offset);
        port.read()
    }

    unsafe fn initialize(&mut self) -> Result<(), String> {
        // Reset drive
        self.write_ctl(0, 0b100);
        // Wait 5 µs
        // Assume each read takes 30 ns
        for _ in 0..5000 / 30 + 1 {
            self.read_ctl(0);
        }
        self.write_ctl(0, 0);

        pic::enable_irq(2);
        idt::register_isr(0x20 + 14, irq14);
        pic::enable_irq(14);

        self.write_io(6, 0xA0u8);
        for _ in 0..16 {
            self.read_ctl(0);
        }
        while self.read_ctl(0) & 0x80 > 0 {}

        println!("Started IDENTIFY for Master");
        for i in 2..=5 {
            self.write_io(i, 0u8);
        }
        println!("Send 0xEC");
        self.write_io(7, 0xECu8);
        if self.read_ctl(0) == 0 {
            println!("ATA disk 0 doesn't exist");
            self.master_drive = NewDiskInfo::Missing;
        } else {
            println!("Waiting for BSY to clear...");
            while self.read_ctl(0) & 0x80 > 0 {}
            let mut identify_response = [0u16; 256];
            if self.read_io::<u8>(4) > 0 || self.read_io::<u8>(5) > 0 {
                println!("Not ATA");
                self.master_drive = NewDiskInfo::Other;
            }
            println!("Waiting for DRQ or ERR to set...");
            loop {
                let ctl = self.read_ctl(0);
                if ctl & 0b1000 > 0 {
                    break;
                } else if ctl & 1 > 0 {
                    println!("ATA disk 0 err during identify");
                    self.master_drive = NewDiskInfo::Other
                }
            }
    
            println!("Reading IDENTIFY response...");
            for value in identify_response.iter_mut() {
                *value = self.read_io(0);
            }
    
            println!("Identity command succeeded");
    
            let master_info = PataDiskInfo {
                supports_lba_48: identify_response[83] & 1 << 10 > 0,
                lba_28_sector_count: identify_response[60] as u32 | (identify_response[61] as u32) << 16,
                lba_48_sector_count: identify_response[100] as u64
                    | (identify_response[101] as u64) << 16
                    | (identify_response[102] as u64) << 32
                    | (identify_response[103] as u64) << 48
            };
            self.master_drive = NewDiskInfo::Pata(master_info);
        }

        crate::ps2::keyboard::get_key();

        println!("Enabling Slave...");
        self.write_io(6, 0xB0u8);
        for _ in 0..16 {
            self.read_ctl(0);
        }
        while self.read_ctl(0) & 0x80 > 0 {}

        println!("Started IDENTIFY for Slave");
        for i in 2..=5 {
            self.write_io(i, 0u8);
        }
        println!("Send 0xEC");
        self.write_io(7, 0xECu8);
        if self.read_ctl(0) == 0 {
            println!("ATA disk 1 doesn't exist");
            self.slave_drive = NewDiskInfo::Missing;
        } else {
            println!("Waiting for BSY to clear...");
            while self.read_ctl(0) & 0x80 > 0 {}
            let mut identify_response = [0u16; 256];
            if self.read_io::<u8>(4) > 0 || self.read_io::<u8>(5) > 0 {
                println!("Not ATA");
                self.master_drive = NewDiskInfo::Other;
            }
            println!("Waiting for DRQ or ERR to set...");
            loop {
                let ctl = self.read_ctl(0);
                if ctl & 0b1000 > 0 {
                    break;
                } else if ctl & 1 > 0 {
                    println!("ATA disk 0 err during identify");
                    self.master_drive = NewDiskInfo::Other
                }
            }
    
            println!("Reading IDENTIFY response...");
            for value in identify_response.iter_mut() {
                *value = self.read_io(0);
            }
    
            println!("Identity command succeeded");
    
            crate::ps2::keyboard::get_key();
    
            let slave_info = PataDiskInfo {
                supports_lba_48: identify_response[83] & 1 << 10 > 0,
                lba_28_sector_count: identify_response[60] as u32 | (identify_response[61] as u32) << 16,
                lba_48_sector_count: identify_response[100] as u64
                    | (identify_response[101] as u64) << 16
                    | (identify_response[102] as u64) << 32
                    | (identify_response[103] as u64) << 48
            };
            self.slave_drive = NewDiskInfo::Pata(slave_info);
        }

        Ok(())
    }

    fn read_sectors(&mut self, drive_select: DiskSelect, start_sector: u64, buffer: &mut [u8]) -> Result<(), String> {
        let drive = match drive_select {
            DiskSelect::Master => match &mut self.master_drive {
                NewDiskInfo::Uninitialized => Err("Master drive not initialized".to_string()),
                NewDiskInfo::Missing => Err("Master drive not connected".to_string()),
                NewDiskInfo::Other => Err("Master drive not ATA".to_string()),
                NewDiskInfo::Pata(info) => Ok(info)
            },
            DiskSelect::Slave => match &mut self.slave_drive {
                NewDiskInfo::Uninitialized => Err("Slave drive not initialized".to_string()),
                NewDiskInfo::Missing => Err("Slave drive not connected".to_string()),
                NewDiskInfo::Other => Err("Slave drive not ATA".to_string()),
                NewDiskInfo::Pata(info) => Ok(info)
            },
        }?;

        if start_sector >= drive.sector_count() {
            return Err(format!("Chosen disk has {} sectors; sector given was {}", drive.sector_count(), start_sector));
        }
        if buffer.len() % 512 != 0 {
            return Err(format!("Buffer length must be multiple of 512 bytes; was {}", buffer.len()));
        }
        let sector_count = buffer.len() as u64 / 512;
        
        if start_sector + sector_count > 0xFFFFFFF || sector_count > 256 {
            if !drive.supports_lba_48 {
                return Err(format!("Sector range {}..{} or sector count {} is out of range for LBA 28", start_sector, start_sector + sector_count, sector_count));
            }
            todo!()
        } else {
            // https://wiki.osdev.org/ATA_PIO_Mode#28_bit_PIO

            
            if sector_count > 256 {
                return Err(format!("Buffer must be max 256*512 bytes; was {}*512", sector_count));
            }
            
            let sector_count = if sector_count == 256 {
                0u8
            } else {
                sector_count as _
            };

            let command = 0xE0 | if drive_select == DiskSelect::Slave { 0x10 } else { 0 } | (start_sector >> 24) as u8 & 0xF;
            unsafe {
                self.write_io(6, command);
                self.write_io(1, 0u8); // Wait
                self.write_io(2, sector_count);
                self.write_io(3, (start_sector & 0xFF) as u8);
                self.write_io(4, (start_sector >> 8 & 0xFF) as u8);
                self.write_io(5, (start_sector >> 16 & 0xFF) as u8);
                self.send_command(0x20, false).unwrap();
                for s in 0..sector_count {
                    self.poll().unwrap();
                    for i in 0..256 {
                        let value: u16 = self.read_io(0);
                        let low = (value & 0xFF) as u8;
                        let high = (value >> 8) as u8;
                        buffer[s as usize * 512 + i * 2] = low;
                        buffer[s as usize * 512 + i * 2 + 1] = high;
                    }
                    self.delay();
                }
            }

            Ok(())
        }
    }
    
    unsafe fn send_command(&mut self, command: u8, poll: bool) -> Result<(), String> {
        let mut command_port = Port::new(self.io_base + 7);
        command_port.write(command);
        if poll { self.poll()?; }
        Ok(())
    }
    
    unsafe fn poll(&mut self) -> Result<(), String> {
        let mut status_port = Port::new(self.ctl_base);
        while status_port.read() & 0x80 > 0 {}
        loop {
            let status: u8 = status_port.read();
            if status & 0x08 > 0 {
                break Ok(())
            }
            else if status & 0x1 > 0 { break Err("ATA error".to_string()) }
            else if status & 0x20 > 0 { break Err("ATA disk error".to_string()) }
        }
    }
    
    unsafe fn delay(&mut self) {
        let mut status_port: Port<u8> = Port::new(self.ctl_base);
        for _ in 0..16 {
            status_port.read();
        }
    }
}

pub enum NewDiskInfo {
    Uninitialized,
    Pata(PataDiskInfo),
    Missing,
    Other
}

pub struct PataDiskInfo {
    supports_lba_48: bool,
    lba_28_sector_count: u32,
    lba_48_sector_count: u64
}

impl PataDiskInfo {
    fn sector_count(&self) -> u64 {
        if self.supports_lba_48 {
            self.lba_48_sector_count
        } else {
            self.lba_28_sector_count as _
        }
    }
}
