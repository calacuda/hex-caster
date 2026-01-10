//! This program launches application of the connected computer when the assisiated "Spell"
//! (pattern) is drawn on the connected touch-pad
//!
//! I2C SCL => Green (GPIO pin 5)
//! I2C SDA => Blue (GPIO pin 4)
//! I2C interupt => Yellow (GPIO pin 3)
//! button (click button) => Orange (GPIO pin 2)

#![no_std]
#![no_main]
#![feature(more_float_constants)]

#[macro_use]
extern crate alloc;

use crate::spell_caster::SpellBuilder;
use crate::spell_compare::process_stroke;
use alloc::vec::Vec;
use core::ptr::addr_of_mut;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_executor::{Executor, Spawner};
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::peripherals::PIN_3;
use embassy_rp::{
    Peri, bind_interrupts, gpio,
    i2c::InterruptHandler as I2cIrqHandler,
    multicore::{Stack, spawn_core1},
    peripherals::{I2C0, PIN_4, PIN_5, USB},
    usb::{Driver, InterruptHandler as UsbIrqHandler},
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Duration, Timer};
use embassy_usb::{
    class::{
        cdc_acm::{CdcAcmClass, State},
        hid::{HidReaderWriter, ReportId, RequestHandler, State as HidState},
    },
    control::OutResponse,
    {Builder, Config, Handler},
};
use embassy_usb_logger::ReceiverHandler;
use embedded_alloc::LlffHeap as Heap;
use gpio::{Level, Output};
use log::*;
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

use {defmt_rtt as _, panic_probe as _};

pub mod spell_caster;
pub mod spell_compare;

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

// static mut CORE1_STACK: Stack<4096> = Stack::new();
static mut CORE1_STACK: Stack<5120> = Stack::new();
// static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbIrqHandler<USB>;
    I2C0_IRQ => I2cIrqHandler<I2C0>;
});

pub type Point = (u16, u16);
pub type SpellId = usize;
pub type Spell = Vec<Point>;
pub type KbdShortcut = Vec<KbdEvent>;

const ADDR: u8 = 0x2c;
// the full report is 37 bytes long but we don't need that much data & the data is generated on
// i2c reads so might as well save some time & only read what we need;
const USB_HID_REPORT_SIZE: usize = 9;

#[global_allocator]
static HEAP: Heap = Heap::empty();
const HEAP_SIZE: usize = 128 * 1024;

// static COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, String, 4> = Channel::new();
static SPELL_CHANNEL: Channel<CriticalSectionRawMutex, Spell, 4> = Channel::new();
static KBD_CHANNEL: Channel<CriticalSectionRawMutex, KeyboardReport, 4> = Channel::new();
static LEARNING: AtomicBool = AtomicBool::new(true);

pub enum KbdEvent {
    Press { scan_code: u8, is_mod: bool },
    Release { scan_code: u8, is_mod: bool },
    Wait(u32),
}

pub struct CmdHandler {
    // learning_mode: Arc<AtomicBool>,
}

impl ReceiverHandler for CmdHandler {
    fn new() -> Self {
        Self {
            // learning_mode: Arc::new(AtomicBool::default()),
        }
    }

    async fn handle_data(&self, data: &[u8]) {
        match core::str::from_utf8(data) {
            Ok(cmd) => {
                info!("recv a command {cmd}");
                // let mut buf = [0u8; 256];
                // COMMAND_CHANNEL.send(cmd.to_string()).await;

                if cmd.starts_with("/greet ") {
                    let name = &cmd[7..cmd.len()];
                    info!("Hello, {name}!");
                } else if cmd.starts_with("/learn") {
                    LEARNING.store(true, Ordering::Relaxed);
                    // Timer::after(Duration::from_millis(3000)).await;
                    info!("entering learn mode");
                } else if cmd.starts_with("/cast") {
                    LEARNING.store(false, Ordering::Relaxed);
                    // Timer::after(Duration::from_millis(3000)).await;
                    info!("entering casting mode");
                } else if cmd.starts_with("/") {
                    error!("unknown command!");
                }
            }
            Err(e) => error!("messeage failed to parse with error: {e}. (likely invalid utf8)"),
        };
    }
}

struct HidRequestHandler {}

impl RequestHandler for HidRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {id:?}");
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {id:?}: {data:?}");
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {id:?} to {dur:?}");
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {id:?}");
        None
    }
}

struct HidDeviceHandler {
    configured: AtomicBool,
}

impl Default for HidDeviceHandler {
    fn default() -> Self {
        Self {
            configured: AtomicBool::new(false),
        }
    }
}

impl HidDeviceHandler {
    fn new() -> Self {
        Self::default()
    }
}

impl Handler for HidDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_heap();

    let p = embassy_rp::init(Default::default());

    // task for serial logging & other usb stuff
    let driver = Driver::new(p.USB, Irqs);
    // spawner.spawn(logger_task(driver)).unwrap();
    spawner
        .spawn(usb_task(driver, KBD_CHANNEL.receiver()))
        .unwrap();

    // LED section
    let led = Output::new(p.PIN_25, Level::Low);
    spawner.spawn(blinky(led)).unwrap();

    // i2c read
    let sda = p.PIN_4;
    let scl = p.PIN_5;
    spawner
        .spawn(trackpad_position(
            p.I2C0,
            sda,
            scl,
            p.PIN_3,
            SPELL_CHANNEL.sender(),
        ))
        .unwrap();
    spawn_core1(
        p.CORE1,
        unsafe { &mut *addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                spawner
                    .spawn(spell_caster(
                        SPELL_CHANNEL.receiver(),
                        KBD_CHANNEL.sender(),
                        // learning,
                    ))
                    .unwrap()
            })
        },
    )
    // .unwrap();
    ;
    Timer::after(Duration::from_millis(1000)).await;

    // info!("Hello, World!");
    info!("all tasks started");
}

#[embassy_executor::task]
async fn spell_caster(
    spell_cast_msg: Receiver<'static, CriticalSectionRawMutex, Spell, 4>,
    kbd_sender: Sender<'static, CriticalSectionRawMutex, KeyboardReport, 4>,
    // learning: Arc<AtomicBool>,
) {
    // will be a Vec<Vec<NormedSpell>> with each Vec<NormedSpell> representing a collection of
    // examples of a spells.
    let mut spells = Vec::new();

    loop {
        let spell_symbol = spell_cast_msg.receive().await;

        if spell_symbol.len() < 5 {
            warn!("Gesture too short, ignoring");
            continue;
        }

        debug!(
            "spell_caster recieved a spell of length {}",
            spell_symbol.len()
        );
        let cast_spell = process_stroke(spell_symbol).await;

        if LEARNING.load(Ordering::Relaxed) {
            spells.push(cast_spell);
            info!("learned a new spell! (spell no. {})", spells.len());

            // LEARNING.store(false, Ordering::Relaxed);
        } else {
            info!("comparing spell to corpus");

            // if let Some(spell) = spells.get(0) {
            // info!("comparing spell of len {}", spell.len());

            // let comp_value = spell_compare::spell_compare(&spell_symbol, &spell).await;
            // info!("comp_value {comp_value}");
            //
            // if comp_value > 0.025 || comp_value.is_nan() {
            //     continue;
            // }

            // let comparisons = spells
            //     .iter()
            //     .map(|spell| spell_compare::maybe_spell_compare(&spell_symbol, &spell));
            // let comp_value = spell_compare::collect_async(comparisons.collect())
            //     .await
            //     .into_iter()
            //     .filter_map(|comp| comp)
            //     .fold(f32::INFINITY, |a, b| if a < b { a } else { b });
            //
            // info!("comp_value: {comp_value}");

            let (spell, comp_value) = spell_compare::spell_compare(cast_spell, &spells).await;
            info!("comp_value: {comp_value}");

            // if comp_value < 0.025 && !comp_value.is_nan() {
            if comp_value > 0.6 && !comp_value.is_nan() {
                warn!("running short cut");
                // let report = KeyboardReport {
                //     modifier: 0x08,
                //     leds: 0,
                //     reserved: 0,
                //     keycodes: [0x28, 0, 0, 0, 0, 0],
                // };
                //
                // kbd_sender.send(report).await;
                //
                // Timer::after(Duration::from_millis(250)).await;
                //
                // let report = KeyboardReport {
                //     keycodes: [0, 0, 0, 0, 0, 0],
                //     leds: 0,
                //     modifier: 0,
                //     reserved: 0,
                // };
                //
                // kbd_sender.send(report).await;
            } else {
                warn!("comparison failed");
            }
        }

        // TODO: match spell against corpus of learned spells
        // TODO: cast spell if known
        // TODO: display error if not.
    }
}

#[embassy_executor::task]
async fn trackpad_position(
    i2c: Peri<'static, I2C0>,
    sda: Peri<'static, PIN_4>,
    scl: Peri<'static, PIN_5>,
    interupt: Peri<'static, PIN_3>,
    spell_caster: Sender<'static, CriticalSectionRawMutex, Spell, 4>,
) {
    info!("starting I2C track pad task");
    let config = embassy_rp::i2c::Config::default();
    let mut bus = embassy_rp::i2c::I2c::new_async(i2c, scl, sda, Irqs, config);
    let mut result: [u8; USB_HID_REPORT_SIZE] = [0u8; USB_HID_REPORT_SIZE];
    let mut spell_builder = SpellBuilder::default();
    let mut int_pin = Input::new(interupt, Pull::None);
    // Enable the schmitt trigger to slightly debounce.
    int_pin.set_schmitt(true);

    loop {
        // int_pin.wait_for_low().await;

        match bus.read_async(ADDR, &mut result).await {
            Ok(_) => {
                // info!("report type = {}", result[2]);
                let report_type = result[2];

                if report_type == 1 {
                    let x = u16::from_le_bytes([result[5], result[6]]);
                    let y = u16::from_le_bytes([result[7], result[8]]);

                    // if (x + y) != 0 {
                    //     debug!("({x}, {y})");
                    // }

                    spell_builder.step((x, y));

                    if spell_builder.should_cast() {
                        spell_caster.send(spell_builder.build()).await;
                        spell_builder.reset();
                    }
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
        // trace!("on");
        // debug!("on");
        Timer::after_millis(250).await;

        led.set_low();
        // trace!("off");
        // debug!("off");
        Timer::after_millis(250).await;
    }
}

#[embassy_executor::task]
async fn usb_task(
    // spawner: Spawner,
    driver: Driver<'static, USB>,
    kbd_shortcuts: Receiver<'static, CriticalSectionRawMutex, KeyboardReport, 4>,
    // learning: Arc<AtomicBool>,
) {
    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Calacuda");
    config.product = Some("Hex-Caster");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.composite_with_iads = false;
    config.device_class = 0;
    config.device_sub_class = 0;
    config.device_protocol = 0;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    // You can also add a Microsoft OS descriptor.
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut request_handler = HidRequestHandler {};
    let mut device_handler = HidDeviceHandler::new();

    let mut logger_state = State::new();
    let mut kbd_state = HidState::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        // &mut [], // no msos descriptors
        &mut msos_descriptor,
        &mut control_buf,
    );

    // Create a class for the logger
    let logger_class = CdcAcmClass::new(&mut builder, &mut logger_state, 64);

    builder.handler(&mut device_handler);

    // Create classes on the builder.
    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut kbd_state, config);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let (reader, mut writer) = hid.split();
    let usb_reader = async {
        reader.run(false, &mut request_handler).await;
    };
    let usb_writer = async {
        loop {
            let report: KeyboardReport = kbd_shortcuts.receive().await;

            // info!("sending report: {report:?}");

            match writer.write_serialize(&report).await {
                Ok(()) => {
                    debug!("report sent successfully");
                }
                Err(e) => warn!("Failed to send report: {:?}", e),
            };
        }
    };

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
        LOGGER.with_handler(CmdHandler {
            // learning_mode: learning,
        });
        let _ = ::log::set_logger_racy(&LOGGER)
            .map(|()| log::set_max_level_racy(log::LevelFilter::Debug));

        LOGGER.create_future_from_class(logger_class)
    };

    // TODO: add other usb handling here
    embassy_futures::join::join4(
        // embassy_futures::join::join_array([log_fut, usb_reader]),
        log_fut, usb_fut, usb_reader, usb_writer,
    )
    .await;
}
