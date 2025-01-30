
use clap::{ArgAction, Parser};
use mcp4xxx::{Mcp4xxx, Tcon, BASE_ADDR};
use linux_embedded_hal::I2cdev;
use tracing::{debug, error, info};
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};

/// MCP4xxx Command Line Utility
#[derive(Clone, Debug, PartialEq, clap::Parser)]
pub struct Args {
    /// I2C device used for connecting
    pub i2c_dev: String,

    /// I2C address for the target device
    #[clap(long, default_value_t=BASE_ADDR)]
    pub i2c_addr: u8,

    #[clap(subcommand)]
    cmd: Command,

    /// Enable verbose logging
    #[clap(long, default_value = "debug")]
    log_level: LevelFilter,
}

#[derive(Clone, Debug, PartialEq, clap::Parser)]
pub enum Command {
    /// Configure a wiper channel
    SetCfg{
        /// P1 channel enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p1_en: bool,

        /// P1 Wiper enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p1_w_en: bool,

        /// P1 Terminal A enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p1_a_en: bool,

        /// P1 Terminal B enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p1_b_en: bool,

        /// P2 channel enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p2_en: bool,

        /// P2 Wiper enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p2_w_en: bool,

        /// P2 Terminal A enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p2_a_en: bool,

        /// P2 Terminal B enable
        #[clap(long, action=ArgAction::Set, default_value_t=true)]
        p2_b_en: bool,
    },
    /// Set a wiper value
    SetWiper{
        /// Channel index (0 or 1 for multi-channel devices)
        #[clap()]
        index: u8,

        value: u16,
    },
}


fn main() -> anyhow::Result<()>{
    // Parse args
    let args = Args::parse();

    // Setup logging
    let filter = EnvFilter::from_default_env().add_directive(args.log_level.into());
    let _ = FmtSubscriber::builder()
        .compact()
        .without_time()
        .with_max_level(args.log_level)
        .with_env_filter(filter)
        .try_init();

    debug!("Args: {args:?}");

    // Connect to I2C bus
    let i2c = I2cdev::new(args.i2c_dev)?;

    debug!("Connecting to device 0x{:02x}", args.i2c_addr);

    // Connect to MCP4xxx device
    let mut mcp = Mcp4xxx::new(args.i2c_addr, i2c)?;

    debug!("Device connected!");

    match args.cmd {
        Command::SetCfg { p1_en, p1_w_en, p1_a_en, p1_b_en, p2_en, p2_w_en, p2_a_en, p2_b_en } => {
            // Map CLI flags to control bits
            let mut tcon = Tcon::empty();
            tcon.set(Tcon::R0HW, p1_en);
            tcon.set(Tcon::R0W, p1_w_en);
            tcon.set(Tcon::R0A, p1_a_en);
            tcon.set(Tcon::R0B, p1_b_en);
            tcon.set(Tcon::R1HW, p2_en);
            tcon.set(Tcon::R1W, p2_w_en);
            tcon.set(Tcon::R1A, p2_a_en);
            tcon.set(Tcon::R1B, p2_b_en);

            mcp.configure(tcon)?;

        },
        Command::SetWiper { index, value } => {
            match index {
                0 => mcp.set_wiper0(value)?,
                1 => mcp.set_wiper1(value)?,
                _ => return Err(anyhow::anyhow!("Invalid index {index}")),
            }
        },
    }

    Ok(())
}
