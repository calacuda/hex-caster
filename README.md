<img src="./assets/demo.gif" alt="Demo GIF" width="100%" height="100%"/>

# Hex-Caster

It's like a streamdeck but more magical.

HexCaster is designed to run on a Raspberry Pi Pico 2. It allows the user to draw symbols on a [Framework Laptop 13 Trackpad](https://frame.work/products/touchpad-kit?v=FRANFT0001) to either

- launch coresponding keyboard shorcuts or,
- run a script on the connected computer.

## How it Works

Framework was kind enough to release their [documentation](https://github.com/FrameworkComputer/Framework-Laptop-13/tree/main/Touchpad), connector pin outs, & a [firmware description](https://github.com/FrameworkComputer/Framework-Laptop-13/blob/main/Touchpad/Firmware.md). This documentation states that the trackpad reports [USB HID (Human Interface Device)](https://en.wikipedia.org/wiki/Human_interface_device) packets over an [I2C bus](https://en.wikipedia.org/wiki/I2C). According to [this hackaday blog post](https://hackaday.com/2024/04/17/human-interfacing-devices-hid-over-i2c/) this is not uncommon. Initially thought my options for micro-controller would be limited becasue the I2C bus operating at 5 Volts. However, after some testing and google searching, it turns out that the bus is 3.3 volt compatible. So I used a Rapberry Pi Pico 2 to read HID packets from the I2C bus and run a path recognition algorithm to trigger keyboiard shorcuts or communicate over serial with a driver running on the computer to run a custom script.

## Project Status

This project is still in developmenet but there is a mostly functional porototype.

## Inspiration

1. [Hexecute](https://github.com/ThatOtherAndrew/Hexecute) => this project is similar but it is a software solution that runs on the computer, and thus has differnt limitation.
2. [touchpad-adventure](https://github.com/jeongm-in/touchpad-adventure) => this repo contains a wealth of helpful links, the nessesary breakout board for the 51 pin ZIF connector (ZIF connector-to-10156001-051100LF) & most importantly to me, documentation that the I2C bus can work at 3.3 volts.
