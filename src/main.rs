//! This example test the RP Pico on board LED.
//!
//! It does not work with the RP Pico W board. See `blinky_wifi.rs`.

#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use alloc::string::{String, ToString};
use embassy_executor::Spawner;
use embassy_rp::{
    peripherals::USB,
    usb::{Driver, InterruptHandler},
    {bind_interrupts, gpio},
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
    USBCTRL_IRQ => InterruptHandler<USB>;
});

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
    let driver = Driver::new(p.USB, Irqs);
    // spawner.spawn(logger_task(driver)).unwrap();
    spawner.spawn(logger_task(driver)).unwrap();

    let led = Output::new(p.PIN_25, Level::Low);
    spawner.spawn(blinky(led)).unwrap();
    // Delay::delay_ms(&mut embassy_time::Delay, 500_u32);
    Timer::after(Duration::from_millis(1000)).await;

    // info!("Hello, World!");
    // loop {
    //     led.set_high();
    //     debug!("on");
    //     Timer::after_millis(250).await;
    //
    //     led.set_low();
    //     debug!("off");
    //     Timer::after_millis(250).await;
    // }
}

#[embassy_executor::task]
async fn blinky(mut led: Output<'static>) {
    loop {
        led.set_high();
        trace!("on");
        Timer::after_millis(250).await;

        led.set_low();
        trace!("off");
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
                } else {
                    // write!(writer, "[{level}] |{}|\r\n", env!("CARGO_PKG_NAME"),).unwrap();
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

// #[embassy_executor::task]
// async fn logger_task(driver: Driver<'static, USB>) {
//     // embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
//     // let class = CdcAcmClass::from(driver);
//     // embassy_usb_logger::with_custom_style!(
//     //     1024,
//     //     log::LevelFilter::Info,
//     //     // CdcAcmClass::new(, , 64),
//     //     // class,
//     //     |record, writer| {
//     //         use core::fmt::Write;
//     //         let level = record.level().as_str();
//     //         write!(writer, "[{level}] {}\r\n", record.args()).unwrap();
//     //     }
//     // );
//
//     static LOGGER: ::embassy_usb_logger::UsbLogger<1024, ::embassy_usb_logger::DummyHandler> =
//         ::embassy_usb_logger::UsbLogger::with_custom_style(|record, writer| {
//             use core::fmt::Write;
//             let level = record.level().as_str();
//             write!(writer, "[{level}] {}\r\n", record.args()).unwrap();
//         });
//     unsafe {
//         let _ = ::log::set_logger_racy(&LOGGER)
//             .map(|()| log::set_max_level_racy(log::LevelFilter::Debug));
//     }
//     let _ = LOGGER
//         .run(&mut ::embassy_usb_logger::LoggerState::new(), driver)
//         .await;
// }
