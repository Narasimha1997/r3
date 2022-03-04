use crate::cpu::io;
extern crate log;
extern crate spin;

use core::fmt;
use io::{wait, Port};

use lazy_static::lazy_static;
use spin::Mutex;

// Command ports are used to send commands.
const MASTER_CMD_PORT: usize = 0x20;
const SLAVE_CMD_PORT: usize = 0xA0;

// Data ports are used to read/write data.
const MASTER_DATA_PORT: usize = 0x21;
const SLAVE_DATA_PORT: usize = 0xA1;

/// rebase hardware IRQs starting from this offset.
const IRQ_OFFSET: u8 = 0x20;

/// command to read Interrupt request register
const CMD_IRQ_REGISTER_READ: u8 = 0x0A;

/// command to read Interrupt service register
const CMD_ISR_REGISTER_READ: u8 = 0x0B;

/// command to init PIC hardware
const CMD_PIC_INIT: u8 = 0x11;

/// Command to set PIC in x86 8086 legacy hardware mode
const DATA_MODE_8086: u8 = 0x01;

/// Command to say interrupt has ended
const CMD_INTERRUPT_ACK: u8 = 0x20;

/// Number of interrupts per PIC chip
const MAX_INTERRUPTS_PER_CHIP: u8 = 8;

pub struct InterruptStatusRegister {
    pub master_isr: u8,
    pub master_irq: u8,
    pub slave_isr: u8,
    pub slave_irq: u8,
}

impl fmt::Display for InterruptStatusRegister {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "master_isr={:08b}, master_irr={:08b}, slave_isr={:08b}, slave_irr={:08b}",
            self.master_isr, self.master_irq, self.slave_isr, self.slave_irq
        )
    }
}

impl fmt::Debug for InterruptStatusRegister {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct PIC {
    pub cmd_port: Port,
    pub data_port: Port,
    pub offset: u8,
}

impl PIC {
    #[inline]
    pub fn can_handle(&self, interrupt_no: u8) -> bool {
        self.offset <= interrupt_no && interrupt_no < self.offset + MAX_INTERRUPTS_PER_CHIP
    }

    #[inline]
    pub fn eoi(&self) {
        self.cmd_port.write_u8(CMD_INTERRUPT_ACK);
    }

    #[inline]
    pub fn init(&self) {
        self.cmd_port.write_u8(CMD_PIC_INIT);
    }

    #[inline]
    pub fn write_cmd(&self, cmd: u8) {
        self.cmd_port.write_u8(cmd);
    }

    #[inline]
    pub fn write_data(&self, data: u8) {
        self.data_port.write_u8(data);
    }

    #[inline]
    pub fn read_cmd(&self) -> u8 {
        self.cmd_port.read_u8()
    }

    pub fn new(cmd_port_no: usize, data_port_no: usize, offset: u8) -> Self {
        PIC {
            cmd_port: Port::new(cmd_port_no, false),
            data_port: Port::new(data_port_no, false),
            offset,
        }
    }
}

pub struct ChainedPIC {
    pub pics: [PIC; 2],
    pub is_enabled: bool,
}

impl ChainedPIC {
    pub fn mask_requests(&self, master_mask: u8, slave_mask: u8) {
        let slave: &PIC = &self.pics[1];
        slave.data_port.write_u8(slave_mask);
        let master: &PIC = &self.pics[0];
        master.data_port.write_u8(master_mask);
    }

    pub fn can_handle(&self, interrupt_no: u8) -> bool {
        let slave: &PIC = &self.pics[1];
        if slave.can_handle(interrupt_no) {
            return true;
        }

        let master: &PIC = &self.pics[0];
        if master.can_handle(interrupt_no) {
            return true;
        }

        false
    }

    pub fn send_eoi(&self, interrupt_no: u8) {
        let slave: &PIC = &self.pics[1];

        if slave.can_handle(interrupt_no) {
            slave.eoi();
        }

        let master: &PIC = &self.pics[0];
        if master.can_handle(interrupt_no) {
            master.eoi();
        }
    }

    pub fn read_registers(&self) -> InterruptStatusRegister {
        let master: &PIC = &self.pics[0];
        let slave: &PIC = &self.pics[1];

        // read master isr and irqs:
        master.write_cmd(CMD_IRQ_REGISTER_READ);
        let master_irq = master.read_cmd();

        master.write_cmd(CMD_ISR_REGISTER_READ);
        let master_isr = master.read_cmd();

        // read slave isr and irqs:
        slave.write_cmd(CMD_IRQ_REGISTER_READ);
        let slave_irq = slave.read_cmd();

        slave.write_cmd(CMD_ISR_REGISTER_READ);
        let slave_isr = slave.read_cmd();

        InterruptStatusRegister {
            master_irq,
            master_isr,
            slave_irq,
            slave_isr,
        }
    }

    pub fn setup(&self, master_mask: u8, slave_mask: u8) {
        // disable (mask) interrupts before set-up
        self.mask_requests(0xff, 0xff);

        let master: &PIC = &self.pics[0];
        let slave: &PIC = &self.pics[1];

        // clear any pending interrupts
        master.eoi();
        wait(1);
        slave.eoi();
        wait(1);

        // init each chip:
        master.init();
        wait(1);
        slave.init();
        wait(1);

        // set offset
        master.write_data(master.offset);
        wait(1);
        slave.write_data(slave.offset);
        wait(1);

        // set chain
        master.write_data(4);
        wait(1);
        slave.write_data(2);
        wait(1);

        // set legacy 8086 mode
        master.write_data(DATA_MODE_8086);
        wait(1);
        slave.write_data(DATA_MODE_8086);
        wait(1);

        // re-mask
        self.mask_requests(master_mask, slave_mask);

        // clear off the interrupts if any:
        master.eoi();
        wait(1);
        slave.eoi();
        wait(1);
    }

    pub fn init(master_mask: u8, slave_mask: u8) -> ChainedPIC {
        let cpcis = ChainedPIC {
            pics: [
                PIC::new(MASTER_CMD_PORT, MASTER_DATA_PORT, IRQ_OFFSET),
                PIC::new(
                    SLAVE_CMD_PORT,
                    SLAVE_DATA_PORT,
                    IRQ_OFFSET + MAX_INTERRUPTS_PER_CHIP,
                ),
            ],
            is_enabled: false,
        };

        cpcis.setup(master_mask, slave_mask);
        cpcis
    }
}

lazy_static! {
    pub static ref CHAINED_PIC: Mutex<ChainedPIC> = Mutex::new(ChainedPIC::init(0xff, 0xff));
}

/// Enables legacy interrupts by clearing the mask bits
/// when enabled, 8259 PIC will raise interrupts on behalf of hardware devices.
pub fn enable_legacy_interrupts() {
    let mut chained_pic = CHAINED_PIC.lock();
    if !chained_pic.is_enabled {
        chained_pic.mask_requests(0x00, 0x00);
        chained_pic.is_enabled = true;
    }
}

/// Disables legacy interrupts by setting the masks.
/// We can disabled PIC once we migrate to LAPIC during SMP boot.
pub fn disable_legacy_interrupts() {
    let mut chained_pic = CHAINED_PIC.lock();
    if chained_pic.is_enabled {
        chained_pic.mask_requests(0xff, 0xff);
        chained_pic.is_enabled = false;
    }
}

pub fn setup_pics() {
    log::info!(
        "PICs initialized in chain PIC mode, n_pics={}",
        CHAINED_PIC.lock().pics.len()
    );
}
