use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mcsaveencrypt-rs")]
pub struct Cli {
    /// Path to save folder or directory containing multiple saves
    pub path: Option<String>,

    /// Force single save mode
    #[arg(short = 's', long)]
    pub single: bool,

    /// Force batch mode (scan subdirectories)
    #[arg(short = 'b', long)]
    pub batch: bool,

    /// Operation: decrypt(0) or encrypt(1) or specific (2,3)
    #[arg(short = 'm', long)]
    pub mode: Option<String>,

    /// Custom hex key (64-bit)
    #[arg(short = 'k', long)]
    pub key: Option<String>,

    /// Output directory
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Pack mode: copy, tar, mcworld (or 0,1,2)
    #[arg(short = 'P', long)]
    pub pack_mode: Option<String>,
}