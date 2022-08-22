# Rust AS3910 driver
This is a no_std driver for the AS3910

This is my first driver. A lot of it is based on https://gitlab.com/jspngh/rfid-rs

Currently support only reading UID from PICCs

Has custom `SpiWithCustomCS` trait to give you control over ChipSelect and ability to implement SPI Lock
