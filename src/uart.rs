use stm32g0::stm32g071::{self, interrupt, Interrupt, NVIC};

use crate::circular_buffer as cbuf;

const BUFF_SIZE: usize = 1024;
static mut USART_RX: cbuf::CircularBuff<u8, BUFF_SIZE> = cbuf::CircularBuff {
    buf: [0u8; BUFF_SIZE],
    wi: 0,
    ri: 0,
};

static mut USART_TX: cbuf::CircularBuff<u8, BUFF_SIZE> = cbuf::CircularBuff {
    buf: [0u8; BUFF_SIZE],
    wi: 0,
    ri: 0,
};

pub fn init() {
    let usart2_r = unsafe { stm32g071::Peripherals::steal().USART2 };
    let gpioa_r = unsafe { stm32g071::Peripherals::steal().GPIOA };

    let clock_r = unsafe { stm32g071::Peripherals::steal().RCC };
    clock_r.iopenr.modify(|_, w| w.iopaen().set_bit());
    clock_r.apbenr1.modify(|_, w| w.usart2en().set_bit());

    // Set RX/TX pins as Outputs
    gpioa_r.moder.modify(unsafe {
        |_, w| {
            w.moder2().bits(0b10);
            w.moder3().bits(0b10)
        }
    });

    // Set RX/TX pins as High-Speed Outputs
    gpioa_r.ospeedr.modify(unsafe {
        |_, w| {
            w.ospeedr3().bits(0b11);
            w.ospeedr2().bits(0b11)
        }
    });

    // RX/TX pins Pull-up
    gpioa_r.pupdr.modify(unsafe {
        |_, w| {
            w.pupdr2().bits(0b01);
            w.pupdr3().bits(0b01)
        }
    });

    // Set alternate fucntions as USART TX/RX
    gpioa_r.afrl.modify(unsafe {
        |_, w| {
            w.afsel2().bits(0b0001);
            w.afsel3().bits(0b0001)
        }
    });

    // set baud-rate to 115200
    let brr = 16_000_000 / 115200;
    usart2_r.brr.write(unsafe { |w| w.bits(brr) });

    unsafe { NVIC::unmask(Interrupt::USART2) }

    usart2_r.cr1.modify(|_, w| {
        w.ue().set_bit();
        w.re().set_bit();
        w.te().set_bit();
        w.rxneie().set_bit();
        w.tcie().set_bit()
    });
}

pub fn logger(buff: &[u8]) {
    let usart2_r = unsafe { stm32g071::Peripherals::steal().USART2 };
    unsafe {
        USART_TX.put_all_data(buff);
    }
    usart2_r.cr1.modify(|_, w| w.tcie().set_bit());
}

pub fn rx_buffer_read() {
    unsafe {
        // TODO put to logger - something like local echo
        USART_RX.get_all_data();
    }
}

#[interrupt]
fn USART2() {
    let usart2_r = unsafe { stm32g071::Peripherals::steal().USART2 };

    // receive buffer not empty isr
    if usart2_r.isr.read().rxne().bit_is_set() {
        let data = usart2_r.rdr.read().bits() as u8;

        unsafe {
            USART_RX.put_data(data);
        }
    }

    // transmission complete isr
    if usart2_r.isr.read().tc().bit_is_set() {
        usart2_r.icr.write(|w| w.tccf().set_bit());

        unsafe {
            if let Some(data) = USART_TX.get_data() {
                usart2_r.tdr.write(|w| w.bits(data as u32));
            } else {
                usart2_r.cr1.modify(|_, w| w.tcie().clear_bit());
            }
        }
    }
}
