use std::fs;

use anyhow::*;
use clap::Parser;
use handlebars::Handlebars;
use rust_embed::RustEmbed;

use eksup::{LatestVersion, TemplateData, Upgrade};

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

fn render(upgrade: Upgrade) -> Result<(), anyhow::Error> {
    let mut handlebars = Handlebars::new();
    handlebars.register_embed_templates::<Templates>().unwrap();

    let tmpl_data = TemplateData::get_data(upgrade);

    // TODO = handlebars should be able to handle backticks and apostrophes
    // Need to figure out why this isn't the case currently
    // let mut output_file = File::create("playbook.md")?;
    let rendered = handlebars.render("playbook.tmpl", &tmpl_data)?;
    // handlebars.render_to_write("playbook.tmpl", &data, &mut output_file)?;

    let replaced = rendered.replace("&#x60;", "`").replace("&#x27;", "'");
    fs::write("playbook.md", replaced)?;

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let args = Upgrade::parse();

    let cluster_version = &args.cluster_version.to_string();
    if LatestVersion.eq(cluster_version) {
        println!(
            "Cluster is already at the latest supported version: {}",
            cluster_version
        );
        println!("Nothing to upgrade at this time");
        return Ok(());
    }

    render(args)?;
    Ok(())
}
