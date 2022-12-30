mod cli;
mod playbook;

use std::process;

use anyhow::*;
use clap::Parser;

pub use cli::{Cli, Commands};

pub const LATEST: &str = "1.24";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::CreatePlaybook(args) => {
            let cluster_version = args.cluster_version.to_string();
            if LATEST.eq(&cluster_version) {
                println!("Cluster is already at the latest supported version: {cluster_version}");
                println!("Nothing to upgrade at this time");
                return Ok(());
            }

            println!("{args:#?}");

            if let Err(err) = playbook::create(args) {
                eprintln!("{err}");
                process::exit(2);
            }
        }

        Commands::Analyze(_args) => {
            todo!();
            // let k8s_client = kube::Client::try_default().await?;
            // analysis::kubernetes::collect_from_nodes(k8s_client).await?;

            // let aws_shared_config = aws_config::load_from_env().await;
            // let aws_client = aws_sdk_eks::Client::new(&aws_shared_config);
            // let cluster = analysis::aws::describe_cluster(&aws_client, &args.cluster_name).await?;
            // println!("{cluster:#?}");
        }
    }

    Ok(())
}
