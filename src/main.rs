use std::fs;
use std::io::Write;

use anyhow::*;
use clap::Parser;
use rust_embed::RustEmbed;

use eksup::Upgrade;

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Asset;

fn main() -> Result<(), anyhow::Error> {
    let args = Upgrade::parse();

    // let eks_version = format!("EKS/versions/{}.md", path_version);

    let path_version = args.cluster_version.to_string().replace('.', "_");
    let index_html = Asset::get(format!("eks/versions/{path_version}.md").as_str()).unwrap();
    let contents = index_html.data.as_ref();

    // println!("{:?}", std::str::from_utf8(index_html.data.as_ref()));

    let file_name = "playbook.md";
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(false)
        .open(file_name)?;
    file.write_all(contents)?;

    Ok(())
}
