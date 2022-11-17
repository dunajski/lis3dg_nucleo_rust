use stm32g0::stm32g071::{self, interrupt, Interrupt, NVIC};

const BUFF_SIZE: usize = 1024;
struct CircularBuff {
    buf: [u16; BUFF_SIZE],
    ri: usize,
    wi: usize,
}

impl CircularBuff {
    fn put_data(&mut self, byte: u16) {
        // set Chip Select to `low` to enable LIS3DH
        let gpiob_r = unsafe { stm32g071::Peripherals::steal().GPIOB };
        gpiob_r.odr.write(|w| w.odr0().clear_bit());

        // p
        self.buf[self.wi] = byte;

        if self.wi == BUFF_SIZE - 1 {
            self.wi = 0;
        } else {
            self.wi += 1;
        }
    }

    fn put_all_data(&mut self, data: &[u16]) {
        for d in data {
            self.put_data(*d);
        }
    }

    fn get_data(&mut self) -> (u16, bool) {
        let mut data_found = false;
        let mut data = 0;
        if self.wi != self.ri {
            data = self.buf[self.ri];
            data_found = true;

            if self.ri == BUFF_SIZE - 1 {
                self.ri = 0;
            } else {
                self.ri += 1;
            }
        }
        (data, data_found)
    }

    fn get_all_data(&mut self) {
        loop {
            let (data, result) = self.get_data();
            if !result {
                break;
            } else {
                put_to_serial(&[data]);
            }
        }
    }
}

static mut TX_CBUF: CircularBuff = CircularBuff {
    buf: [0; BUFF_SIZE],
    wi: 0,
    ri: 0,
};
static mut RX_CBUF: CircularBuff = CircularBuff {
    buf: [0; BUFF_SIZE],
    wi: 0,
    ri: 0,
};

// MISO/D12 PA6 SPI_1_MISO
// MOSI/D11 PA7 SPI_1_MOSI
// CS/D10 PB0 SPI CS
// SCK/A1 PA1 SPI_1_SCK

pub fn init() {
    let spi_r = unsafe { stm32g071::Peripherals::steal().SPI1 };
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
    ///////
    gpiob_r
        .moder
        .modify(unsafe { |_, w| w.moder0().bits(0b01) });

    // MISO/MOSI/SCK high speed io
    gpiob_r
        .ospeedr
        .modify(unsafe { |_, w| w.ospeedr0().bits(0b11) });

    // MISO/MOSI/SCK as Push Pull io
    gpiob_r
        .pupdr
        .modify(unsafe { |_, w| w.pupdr0().bits(0b00) });

    // SET CS to HIGH - after init it should be logic '1' and to read/write set to '0'
    gpiob_r.odr.write(|w| w.odr0().set_bit());

    // 16 000 000 / 2 = 8 MHz
    unsafe {
        spi_r.cr1.modify(|_, w| w.br().bits(0b00));
    }

    spi_r.cr1.modify(|_, w| {
        w.mstr().set_bit(); // uC is Master SPI
        w.spe().set_bit(); // SPI Enable
        w.ssm().set_bit();
        w.ssi().set_bit();
        w.cpol().set_bit(); // CK '1' when idle
        w.cpha().set_bit() // Second clock transition is the first data capture edge
    });

    spi_r.cr2.modify(|_, w| {
        unsafe { w.ds().bits(0b0111) }; // Data-size 8b
                                        // w.txeie().set_bit(); // TX Empty ISR Enable
        w.rxneie().set_bit() // RX Not Empty ISR Enable
    });
    unsafe { NVIC::unmask(Interrupt::SPI1) }
}

#[interrupt]
fn SPI1() {
    let spi1_r = unsafe { stm32g071::Peripherals::steal().SPI1 };

    if spi1_r.sr.read().rxne().bit_is_set() {
        let data = spi1_r.dr.read().bits() as u16;
        unsafe {
            RX_CBUF.put_data(data);
            // 0b0011001100110011
        }
    }

    // if spi1_r.sr.read().txe().bit_is_set() {
    //     let data = spi1_r.dr.read().bits() as u16;
    //     unsafe {
    //         RX_CBUF.put_data(data);
    //     }
    // }

    // if spi1_r.sr.read().tifrfe()
    // if usart2_r.isr.read().tc().bit_is_set() {
    //     usart2_r.icr.write(|w| w.tccf().set_bit());

    //     unsafe {
    //         let (byte, result) = TX_CBUF.get_byte();
    //         if result == false {
    //             usart2_r.cr1.modify(|_, w| w.tcie().clear_bit());
    //         } else {
    //             usart2_r.tdr.write(|w| w.bits(byte as u32));
    //         }
    //     }
    // }
}

pub fn put_to_serial(buff: &[u16]) {
    let spi1_r = unsafe { stm32g071::Peripherals::steal().SPI1 };
    unsafe {
        TX_CBUF.put_all_data(buff);
        let (data, _) = TX_CBUF.get_data();

        spi1_r.dr.write(|w| w.bits(data as u32));

        // spi1_r.cr2.write(|w| w.txeie().set_bit());
    }
}

pub fn rx_buffer_read() {
    unsafe {
        RX_CBUF.get_data();
    }
}
