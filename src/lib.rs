//! Driver for MCP454X/456X/464X/466X I2C variable potentiometers
//! 
#![no_std]

#[cfg(feature = "std")]
extern crate std;

use embedded_hal::i2c::{ErrorType, I2c};
use modular_bitfield::prelude::*;

#[cfg(feature = "defmt")]
use defmt::{debug, error, bitflags};

#[cfg(feature = "tracing")]
use tracing::{debug, error};

#[cfg(not(feature = "defmt"))]
use bitflags::bitflags;


/// Base address for the MCP4661, set the lower 3 bits to match the device A[0:2] pins
pub const BASE_ADDR: u8 = 0b0101000;

// Setup error bounds depending on enabled features
cfg_if::cfg_if! {
    if #[cfg(all(feature = "thiserror", feature = "defmt"))] {
        /// Error bounds when `thiserror` and `defmt` features are enabled
        pub trait ErrorBounds: core::error::Error + defmt::Format {}
        impl <T: core::error::Error + defmt::Format > ErrorBounds for T {}
    } else if #[cfg(feature = "thiserror")] {
        /// Error bounds when `thiserror` feature is enabled
        pub trait ErrorBounds: core::error::Error {}
        impl <T: core::error::Error > ErrorBounds for T {}
    } else if #[cfg(feature = "defmt")] {
        /// Error bounds when `defmt` feature is enabled
        pub trait ErrorBounds: defmt::Format {}
        impl <T: defmt::Format > ErrorBounds for T {}
    } else {
        /// Error bounds when `thiserror` and `defmt` features are disabled
        pub trait ErrorBounds {}
        impl <T> ErrorBounds for T {}
    }
}

/// Mcp6xxx device
pub struct Mcp4xxx<I: I2c> {
    addr: u8,
    i2c: I,
}

/// Register addresses
#[repr(u8)]
pub enum Regs {
    /// Wiper 0 register
    Wiper0 = 0x00,
    /// Wiper 1 register
    Wiper1 = 0x01,
    /// Control register
    Tcon = 0x04,
}

/// Command encoding
#[bitfield]
#[derive(BitfieldSpecifier, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub struct Command {
    /// Wiper MSB
    pub msb: B2,
    /// Register operation
    pub operation: Op,
    /// Register address
    pub address: B4,
}

/// I2C operation
#[derive(BitfieldSpecifier, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[bits = 2]
pub enum Op {
    Write = 0b00,
    Increment = 0b01,
    Decrement = 0b10,
    Read = 0b11,
}

bitflags! {
    /// TCON register flags
    ///
    /// See datasheet table REGISTER 4-1: TCON BITS (ADDRESS = 0x04)
    #[cfg_attr(not(feature = "defmt"), derive(Clone, PartialEq, Debug))]
    pub struct Tcon: u16 {
        /// Enable general call commands
        const GCEN = (1 << 8);
        /// Resistor 1 Hardware Control (1 for enabled)
        const R1HW = (1 << 7);
        /// Resistor 1 Terminal A connect
        const R1A = (1 << 6);
        /// Resistor 1 Wiper connect
        const R1W = (1 << 5);
        /// Resistor 1 Terminal B connect
        const R1B = (1 << 4);
        /// Resistor 1 Hardware Control (1 for enabled)
        const R0HW = (1 << 3);
        /// Resistor 1 Terminal A connect
        const R0A = (1 << 2);
        /// Resistor 1 Wiper connect
        const R0W = (1 << 1);
        /// Resistor 1 Terminal B connect
        const R0B = (1 << 0);

        /// Helper to enable resistor 1
        const R1ALL = Self::R1HW.bits() | Self::R1A.bits() | Self::R1B.bits() | Self::R1W.bits();

        /// Helper to enable resistor 0
        const R0ALL = Self::R0HW.bits() | Self::R0A.bits() | Self::R0B.bits() | Self::R0W.bits();

        const R01 = Self::R0ALL.bits() | Self::R1ALL.bits();
    }
}

/// Mcp4xxx errors
#[derive(Clone, Debug)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Mcp4xxxError<E: ErrorBounds> {
    #[cfg_attr(feature = "thiserror", error("Configuration failed"))]
    Config,
    #[cfg_attr(feature = "thiserror", error("I2C error {0}"))]
    I2c(E),
}

impl<I: I2c> Mcp4xxx<I>
where
    <I as ErrorType>::Error: ErrorBounds,
{
    /// Create a new MCP6441 instance using the provided I2C interface
    pub fn new(addr: u8, mut i2c: I) -> Result<Self, Mcp4xxxError<<I as ErrorType>::Error>> {
        // Attempt to read from device
        let mut buff = [0u8; 1];
        match i2c.read(addr, &mut buff) {
            Ok(_) => {
                #[cfg(any(feature = "tracing", feature = "defmt"))]
                debug!("MCP4661 (0x{:x}) read OK!", addr);
            },
            Err(e) => {
                #[cfg(any(feature = "tracing", feature = "defmt"))]
                error!("MCP4661 (0x{:x}) I2C comms failed: {}", addr, e);
                return Err(Mcp4xxxError::I2c(e));
            }
        }

        let s = Self { addr, i2c };

        // TODO: encode ranges / centre values

        Ok(s)
    }

    /// Configure the device using the TCON register
    pub fn configure(&mut self, tcon: Tcon) -> Result<(), Mcp4xxxError<<I as ErrorType>::Error>> {
        #[cfg(any(feature = "tracing", feature = "defmt"))]
        debug!("MCP4661 CFG {:?}", tcon);

        // Write TCON register
        self.write(Regs::Tcon as u8, tcon.bits())?;

        // Read back TCON value
        let read = self.read(Regs::Tcon as u8)?;
        let tcon_read = Tcon::from_bits_truncate(read);

        if tcon != tcon_read {
            #[cfg(any(feature = "tracing", feature = "defmt"))]
            error!("TCON write mismatch wrote {:?} received {:?}", tcon, tcon_read);
            return Err(Mcp4xxxError::Config);
        }

        Ok(())
    }

    /// Set wiper0 position
    pub fn set_wiper0(&mut self, val: u16) -> Result<(), Mcp4xxxError<<I as ErrorType>::Error>> {
        self.write(Regs::Wiper0 as u8, val)
    }

    /// Set wiper1 position
    pub fn set_wiper1(&mut self, val: u16) -> Result<(), Mcp4xxxError<<I as ErrorType>::Error>> {
        self.write(Regs::Wiper1 as u8, val)
    }

    /// Write to a device register
    pub fn write(
        &mut self,
        reg: u8,
        data: u16,
    ) -> Result<(), Mcp4xxxError<<I as ErrorType>::Error>> {
        let cmd = Command::new()
            .with_address(reg)
            .with_operation(Op::Write)
            .with_msb((data >> 8) as u8 & 0b11);

        let wr = [cmd.bytes[0], data as u8];

        #[cfg(feature = "defmt")]
        debug!("MCP4661 write {:02x}", wr);
        #[cfg(feature = "tracing")]
        debug!("MCP4661 write {:02x?}", wr);

        match self.i2c.write(self.addr, &wr) {
            Ok(_) => {
                #[cfg(any(feature = "tracing", feature = "defmt"))]
                debug!("MCP4661 ({:02x}) write OK!", self.addr);
            },
            Err(e) => {
                #[cfg(any(feature = "tracing", feature = "defmt"))]
                error!("MCP4661 ({:02x}) I2C comms failed: {}", self.addr, e);
                return Err(Mcp4xxxError::I2c(e));
            }
        }

        Ok(())
    }

    /// Read from a device register
    pub fn read(&mut self, reg: u8) -> Result<u16, Mcp4xxxError<<I as ErrorType>::Error>> {
        let cmd = Command::new().with_address(reg).with_operation(Op::Read);

        let mut buff = [0u8; 2];
        match self.i2c.write_read(self.addr, &cmd.bytes, &mut buff) {
            Ok(_) => {
                #[cfg(feature = "defmt")]
                debug!("MCP4661 (0x{:02x}) read: {:02x}", self.addr, buff);
                #[cfg(feature = "tracing")]
                debug!("MCP4661 (0x{:02x}) read: {:02x?}", self.addr, buff);
            },
            Err(e) => {
                #[cfg(any(feature = "tracing", feature = "defmt"))]
                error!("MCP4661 (0x{:02x}) I2C comms failed: {}", self.addr, e);
                return Err(Mcp4xxxError::I2c(e));
            }
        }

        Ok((buff[0] as u16) << 8 | buff[1] as u16)
    }
}
