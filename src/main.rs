use anyhow::*;
use clap::Parser;

use eksup::{render, Upgrade};

fn main() -> Result<(), anyhow::Error> {
    let args = Upgrade::parse();

    render(args)?;

    Ok(())
}
