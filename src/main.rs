use std::fs;
use std::process;

use anyhow::*;
use clap::Parser;
use handlebars::{to_json, Handlebars};
use rust_embed::RustEmbed;

use eksup::{Commands, Compute, LatestVersion, Playbook, TemplateData, Upgrade};

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

fn render(playbook: &Playbook) -> Result<(), anyhow::Error> {
    // Registry templates with handlebars
    let mut handlebars = Handlebars::new();
    handlebars.register_embed_templates::<Templates>().unwrap();

    let mut tmpl_data = TemplateData::get_data(playbook.clone());

    // Render sub-templates for data plane components
    if playbook.compute.contains(&Compute::EksManaged) {
        tmpl_data.insert(
            "eks_managed_node_group".to_string(),
            to_json(handlebars.render("data-plane/eks-managed-node-group.md", &tmpl_data)?),
        );
    }
    if playbook.compute.contains(&Compute::SelfManaged) {
        tmpl_data.insert(
            "self_managed_node_group".to_string(),
            to_json(handlebars.render("data-plane/self-managed-node-group.md", &tmpl_data)?),
        );
    }
    if playbook.compute.contains(&Compute::FargateProfile) {
        tmpl_data.insert(
            "fargate_profile".to_string(),
            to_json(handlebars.render("data-plane/fargate-profile.md", &tmpl_data)?),
        );
    }

    // println!("{:#?}", tmpl_data);

    // TODO = handlebars should be able to handle backticks and apostrophes
    // Need to figure out why this isn't the case currently
    // let mut output_file = File::create("playbook.md")?;
    let rendered = handlebars.render("playbook.md", &tmpl_data)?;
    // handlebars.render_to_write("playbook.tmpl", &data, &mut output_file)?;
    let replaced = rendered
        .replace("&#x60;", "`")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&amp;lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x3D;", "=");
    fs::write(&playbook.filename, replaced)?;

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Upgrade::parse();

    match &cli.command {
        Commands::CreatePlaybook(args) => {
            let cluster_version = args.cluster_version.to_string();
            if LatestVersion.eq(&cluster_version) {
                println!("Cluster is already at the latest supported version: {cluster_version}");
                println!("Nothing to upgrade at this time");
                return Ok(());
            }

            if let Err(err) = render(args) {
                eprintln!("{err}");
                process::exit(2);
            }
        }

        Commands::Analyze(args) => {
            println!("{args:?}");
            // if let Err(err) = render(args) {
            //     eprintln!("{err}");
            //     process::exit(2);
            // }
        }
    }

    Ok(())
}
