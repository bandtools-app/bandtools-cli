pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod output;

use anyhow::Result;
use clap::Parser;

pub fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    commands::run(cli)
}
