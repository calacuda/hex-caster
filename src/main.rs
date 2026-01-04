//! This example test the RP Pico on board LED.
//!
//! It does not work with the RP Pico W board. See `blinky_wifi.rs`.

#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayMs;
use embassy_executor::Spawner;
use embassy_rp::{
    peripherals::USB,
    usb::{Driver, InterruptHandler},
    {bind_interrupts, gpio},
};
use embassy_time::{Delay, Duration, Timer};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    {Builder, Config},
};
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

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    // spawner.spawn(logger_task(driver)).unwrap();
    spawner.spawn(logger_task(driver)).unwrap();

    let mut led = Output::new(p.PIN_25, Level::Low);
    // Delay::delay_ms(&mut embassy_time::Delay, 500_u32);
    Timer::after(Duration::from_millis(1000)).await;

    info!("Hello, World!");
    loop {
        led.set_high();
        debug!("on");
        Timer::after_millis(250).await;

        led.set_low();
        debug!("off");
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

    let log_fut = embassy_usb_logger::with_custom_style!(
        1024,
        log::LevelFilter::Debug,
        logger_class,
        |record, writer| {
            use core::fmt::Write;
            let level = record.level().as_str();
            write!(writer, "[{level}] {}\r\n", record.args()).unwrap();
        }
    );
    // log_fut.await
    embassy_futures::join::join(log_fut, usb_fut).await;
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
