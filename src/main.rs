use std::process;

use anyhow::*;
use clap::Parser;

use eksup::{playbook, Cli, Commands};

pub const LATEST: &str = "1.24";

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::CreatePlaybook(args) => {
            let cluster_version = args.cluster_version.to_string();
            if LATEST.eq(&cluster_version) {
                println!("Cluster is already at the latest supported version: {cluster_version}");
                println!("Nothing to upgrade at this time");
                return Ok(());
            }

            if let Err(err) = playbook::create(args) {
                eprintln!("{err}");
                process::exit(2);
            }
        }

        Commands::Analyze(args) => {
            println!("{args:?}");
        }
    }

    Ok(())
}
