//! This program launches application of the connected computer when the assisiated "Spell"
//! (pattern) is drawn on the connected touch-pad
//!
//! I2C SCL => Green (GPIO pin 5)
//! I2C SDA => Blue (GPIO pin 4)
//! I2C interupt => Yellow (GPIO pin 3)
//! button (click button) => Orange (GPIO pin 2)

#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use alloc::string::{String, ToString};
use embassy_executor::Spawner;
use embassy_rp::{
    Peri, bind_interrupts, gpio,
    i2c::InterruptHandler as I2cIrqHandler,
    peripherals::{I2C0, PIN_4, PIN_5, USB},
    usb::{Driver, InterruptHandler as UsbIrqHandler},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    {Builder, Config},
};
use embassy_usb_logger::ReceiverHandler;
use embedded_alloc::LlffHeap as Heap;
use gpio::{Level, Output};
use log::*;
use {defmt_rtt as _, panic_probe as _};

// Program metadata for `picotool info`.
// This isn't needed, but it's recomended to have these minimal entries.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Hex-Caster"),
    embassy_rp::binary_info::rp_program_description!(c"magical streamdeck"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

#[allow(static_mut_refs)]
fn init_heap() {
    use core::mem::MaybeUninit;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
}

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbIrqHandler<USB>;
    I2C0_IRQ => I2cIrqHandler<I2C0>;
});

const ADDR: u8 = 0x2c;
// the full report is 37 bytes long but we don't need that much data & the data is generated on
// i2c reads so might as well save some time & only read what we need;
const USB_HID_REPORT_SIZE: usize = 9;

#[global_allocator]
static HEAP: Heap = Heap::empty();
const HEAP_SIZE: usize = 128 * 1024;

static COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, String, 4> = Channel::new();

pub struct CmdHandler {}

impl ReceiverHandler for CmdHandler {
    fn new() -> Self {
        Self {}
    }

    async fn handle_data(&self, data: &[u8]) {
        match core::str::from_utf8(data) {
            Ok(cmd) => {
                info!("recv a command {cmd}");
                // let mut buf = [0u8; 256];
                COMMAND_CHANNEL.send(cmd.to_string()).await;

                if cmd.starts_with("/greet ") {
                    let name = &cmd[7..cmd.len()];

                    info!("Hello, {name}!");
                }
            }
            Err(e) => error!("messeage failed to parse with error: {e}. (likely invalid utf8)"),
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_heap();

    let p = embassy_rp::init(Default::default());

    // task for serial logging & other usb stuff
    let driver = Driver::new(p.USB, Irqs);
    // spawner.spawn(logger_task(driver)).unwrap();
    spawner.spawn(logger_task(driver)).unwrap();

    // LED section
    let led = Output::new(p.PIN_25, Level::Low);
    spawner.spawn(blinky(led)).unwrap();

    // i2c read
    let sda = p.PIN_4;
    let scl = p.PIN_5;
    // let config = embassy_rp::i2c::Config::default();
    // let mut bus = embassy_rp::i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, config);
    spawner.spawn(trackpad_position(p.I2C0, sda, scl)).unwrap();
    Timer::after(Duration::from_millis(1000)).await;

    // info!("Hello, World!");
    info!("all tasks started");
}

#[embassy_executor::task]
async fn trackpad_position(
    i2c: Peri<'static, I2C0>,
    sda: Peri<'static, PIN_4>,
    scl: Peri<'static, PIN_5>,
) {
    info!("starting I2C track pad task");
    let config = embassy_rp::i2c::Config::default();
    let mut bus = embassy_rp::i2c::I2c::new_async(i2c, scl, sda, Irqs, config);
    let mut result: [u8; USB_HID_REPORT_SIZE] = [0u8; USB_HID_REPORT_SIZE];

    loop {
        match bus.read_async(ADDR, &mut result).await {
            Ok(_) => {
                // info!("report type = {}", result[2]);
                let report_type = result[2];

                if report_type == 1 {
                    let x = u16::from_le_bytes([result[5], result[6]]);
                    let y = u16::from_le_bytes([result[7], result[8]]);

                    // info!("({x}, {y})");
                }
            }
            Err(e) => error!("could not read from i2c. attempt failed with error: {e:?}"),
        }
    }
}

#[embassy_executor::task]
async fn blinky(mut led: Output<'static>) {
    loop {
        led.set_high();
        trace!("on");
        // debug!("on");
        Timer::after_millis(250).await;

        led.set_low();
        trace!("off");
        // debug!("off");
        Timer::after_millis(250).await;
    }
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut logger_state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    // Create a class for the logger
    let logger_class = CdcAcmClass::new(&mut builder, &mut logger_state, 64);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    // let log_fut = embassy_usb_logger::with_custom_style!(
    //     1024,
    //     log::LevelFilter::Debug,
    //     logger_class,
    //     |record, writer| {
    //         use core::fmt::Write;
    //         let level = record.level().as_str();
    //         write!(writer, "[{level}] {}\r\n", record.args()).unwrap();
    //     }
    // );
    #[allow(static_mut_refs)]
    let log_fut = unsafe {
        static mut LOGGER: ::embassy_usb_logger::UsbLogger<1024, CmdHandler> =
            ::embassy_usb_logger::UsbLogger::with_custom_style(|record, writer| {
                use core::fmt::Write;
                let level = record.level().as_str();

                if record
                    .target()
                    .starts_with(&env!("CARGO_PKG_NAME").replace("-", "_"))
                {
                    write!(writer, "[{level}] {}\r\n", record.args(),).unwrap();
                }
            });
        LOGGER.with_handler(CmdHandler::new());
        let _ = ::log::set_logger_racy(&LOGGER)
            .map(|()| log::set_max_level_racy(log::LevelFilter::Debug));

        LOGGER.create_future_from_class(logger_class)
    };

    // log_fut.await

    // TODO: add other usb handling here
    embassy_futures::join::join(
        // futures that reutrn -> ()
        embassy_futures::join::join_array([log_fut]),
        // futures that reutrn -> !
        embassy_futures::join::join_array([usb_fut]),
    )
    .await;
}
