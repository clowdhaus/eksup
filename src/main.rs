mod aws;
mod check;
mod cli;
mod k8s;
mod playbook;

use std::process;

use anyhow::*;
use clap::Parser;
use cli::{Cli, Commands};

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

      if let Err(err) = playbook::create(args) {
        eprintln!("{err}");
        process::exit(2);
      }
    }

    Commands::Analyze(args) => {
      // // Query Kubernetes first so that we can get AWS details that require further querying
      let k8s_client = kube::Client::try_default().await?;

      let aws_shared_config = aws::get_shared_config(args.region.clone()).await;
      let eks_client = aws_sdk_eks::Client::new(&aws_shared_config);
      let ec2_client = aws_sdk_ec2::Client::new(&aws_shared_config);
      let asg_client = aws_sdk_autoscaling::Client::new(&aws_shared_config);

      let cluster = aws::get_cluster(&eks_client, &args.cluster_name).await?;
      // println!("{cluster:#?}");

      if false {
        let control_plane_subnet_ids = cluster
          .resources_vpc_config()
          .unwrap()
          .subnet_ids
          .as_ref()
          .unwrap();
        let control_plane_subnet_ids =
          aws::get_subnets(&ec2_client, control_plane_subnet_ids.clone()).await?;
        println!("{control_plane_subnet_ids:#?}");

        let eks_managed_node_groups =
          aws::get_eks_managed_node_groups(&eks_client, &args.cluster_name).await?;
        println!("EKS MNG:{eks_managed_node_groups:#?}");

        let self_managed_node_groups =
          aws::get_self_managed_node_groups(&asg_client, &args.cluster_name).await?;
        println!("Self MNG:{self_managed_node_groups:#?}");

        let fargate_profiles = aws::get_fargate_profiles(&eks_client, &args.cluster_name).await?;
        println!("Fargate:{fargate_profiles:#?}");
      }

      let nodes = k8s::get_nodes(&k8s_client).await?;
      println!("Nodes:{nodes:#?}");

      let cluster_version = &cluster.version.unwrap();
      println!("{cluster_version}");
      // println!("{}", args.cluster_version.major());
      // println!("{}", args.cluster_version.minor());
    }
  }

  Ok(())
}
