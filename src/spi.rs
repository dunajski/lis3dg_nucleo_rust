use stm32g0::stm32g071::{self, interrupt, Interrupt, NVIC};

use crate::{circular_buffer as cbuf, uart};

const BUFF_SIZE: usize = 1024;
static mut SPI_RX: cbuf::CircularBuff<u8, BUFF_SIZE> = cbuf::CircularBuff {
    buf: [0u8; BUFF_SIZE],
    wi: 0,
    ri: 0,
};

static mut SPI_TX: cbuf::CircularBuff<u8, BUFF_SIZE> = cbuf::CircularBuff {
    buf: [0u8; BUFF_SIZE],
    wi: 0,
    ri: 0,
};

fn set_cs_pin_low() {
    let gpiob_r = unsafe { stm32g071::Peripherals::steal().GPIOB };
    gpiob_r.odr.write(|w| w.odr0().clear_bit());
}

// MISO/D12 PA6 SPI_1_MISO
// MOSI/D11 PA7 SPI_1_MOSI
// CS/D10 PB0 SPI CS
// SCK/A1 PA1 SPI_1_SCK

pub fn init() {
    let spi1_r = unsafe { stm32g071::Peripherals::steal().SPI1 };
    let gpioa_r = unsafe { stm32g071::Peripherals::steal().GPIOA };
    let gpiob_r = unsafe { stm32g071::Peripherals::steal().GPIOB };

    let clock_r = unsafe { stm32g071::Peripherals::steal().RCC };
    clock_r.iopenr.modify(|_, w| w.iopaen().set_bit());
    clock_r.apbenr2.modify(|_, w| w.spi1en().set_bit());
    clock_r.iopenr.modify(|_, w| w.iopben().set_bit());

    // Set MISO/MOSI/SCK as Alternate Function IO
    gpioa_r.moder.modify(unsafe {
        |_, w| {
            w.moder6().bits(0b10);
            w.moder7().bits(0b10);
            w.moder1().bits(0b10)
        }
    });

    // MISO/MOSI/SCK high speed io
    gpioa_r.ospeedr.modify(unsafe {
        |_, w| {
            w.ospeedr6().bits(0b11);
            w.ospeedr7().bits(0b11);
            w.ospeedr1().bits(0b11)
        }
    });

    // MISO/MOSI/SCK as Push Pull io
    gpioa_r.pupdr.modify(unsafe {
        |_, w| {
            w.pupdr6().bits(0b00);
            w.pupdr7().bits(0b00);
            w.pupdr1().bits(0b00)
        }
    });

    // Set alternate fucntions MISO/MOSI/SCK
    gpioa_r.afrl.modify(unsafe {
        |_, w| {
            w.afsel6().bits(0b0000);
            w.afsel7().bits(0b0000);
            w.afsel1().bits(0b0000)
        }
    });

    // CS as General Purpouse Output...
    gpiob_r
        .moder
        .modify(unsafe { |_, w| w.moder0().bits(0b01) });

    // ... CS as high speed io...
    gpiob_r
        .ospeedr
        .modify(unsafe { |_, w| w.ospeedr0().bits(0b11) });

    // ... CS no Pull-up/Pull-down
    gpiob_r
        .pupdr
        .modify(unsafe { |_, w| w.pupdr0().bits(0b00) });

    // SET CS to HIGH - after init it should be logic '1' and to read/write set to '0'
    gpiob_r.odr.write(|w| w.odr0().set_bit());

    // 16 000 000 / 2 = 8 MHz
    unsafe {
        spi1_r.cr1.modify(|_, w| w.br().bits(0b00));
    }

    spi1_r.cr1.modify(|_, w| {
        w.mstr().set_bit(); // uC is Master SPI
        w.spe().set_bit(); // SPI Enable
        w.ssm().set_bit(); // Software Slave Managment
        w.ssi().set_bit(); // Internal Slave Select
        w.cpol().set_bit(); // CK '1' when idle
        w.cpha().set_bit() // Second clock transition is the first data capture edge
    });

    spi1_r.cr2.modify(|_, w| {
        unsafe { w.ds().bits(0b0111) }; // Data-size 8b
        w.frxth().set_bit(); // FIFO reception threshold 8 bit
        w.rxneie().set_bit() // RX Not Empty ISR Enable
    });
    unsafe { NVIC::unmask(Interrupt::SPI1) }
}

#[interrupt]
fn SPI1() {
    let spi1_r = unsafe { stm32g071::Peripherals::steal().SPI1 };
    let sr_r = spi1_r.sr.read();

    if sr_r.rxne().bit_is_set() {
        let data = spi1_r.dr.read().bits() as u8;
        unsafe {
            SPI_RX.put_data(data);
            uart::logger(&[data]);
        }
    }

    if sr_r.txe().bit_is_set() {
        unsafe {
            if let Some(data) = SPI_TX.get_data() {
                spi1_r.dr.write(|w| w.bits(data as u32));
            } else {
                spi1_r.cr2.modify(|_, w| {
                    w.txeie().clear_bit() // TX Empty ISR DISABLE
                });
            }
        }
    }
}

pub fn logger(buff: &[u8]) {
    set_cs_pin_low();

    let spi1_r = unsafe { stm32g071::Peripherals::steal().SPI1 };

    unsafe {
        SPI_TX.put_all_data(buff);
    }
    spi1_r.cr2.modify(|_, w| {
        w.txeie().set_bit() // TX Empty ISR Enable
    });
}

pub fn rx_buffer_read() {
    unsafe {
        // TODO from here frames should be redirected to parse
        SPI_RX.get_all_data();
    }
}
