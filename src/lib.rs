#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "hal_1_0")]
pub mod as3910;
#[cfg(feature = "hal_0_2")]
pub mod as3910_2_7;
pub mod command;
pub mod register;
