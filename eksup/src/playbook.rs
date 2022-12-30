use std::fs;

use config::{Config, ConfigError, File};
use handlebars::{to_json, Handlebars};
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::value::{Map, Value as Json};

use crate::cli::{Compute, Playbook};

/// Data related to Amazon EKS service
#[derive(Deserialize, Debug)]
pub struct Eks {}

/// Data related to Kuberenetes open source project (upstream)
#[derive(Deserialize, Debug)]
pub struct Kubernetes {
    pub release_url: String,
    pub deprecation_url: Option<String>,
}

/// Configuration data that will be passed to the template(s) rendered
#[derive(Deserialize, Debug)]
pub struct TemplateData {
    pub eks: Eks,
    pub kubernetes: Kubernetes,
}

/// Load configuration data from the associated Kubernetes version data file
#[allow(unused)]
impl TemplateData {
    pub fn new(file_with_name: String) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name(&file_with_name).required(false))
            .build()?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_deserialize()
    }

    pub fn get_data(playbook: Playbook) -> Map<String, Json> {
        let mut tmpl_data = Map::new();

        tmpl_data.insert("cluster_name".to_string(), to_json(playbook.cluster_name));

        let version = playbook.cluster_version.to_string();

        // Parse out minor version string into an integer
        let current_minor_version = version.split('.').collect::<Vec<&str>>()[1]
            .parse::<i32>()
            .unwrap();

        let target_version = format!("1.{}", current_minor_version + 1);
        let config_data = TemplateData::new(format!("templates/data/{target_version}.toml"))
            .unwrap_or_else(|_| {
                panic!("EKS does not support Kubernetes v{target_version} at this time")
            });

        tmpl_data.insert("current_version".to_string(), to_json(version));
        tmpl_data.insert("target_version".to_string(), to_json(target_version));

        let deprecation_url = match config_data.kubernetes.deprecation_url {
            Some(url) => url,
            None => "".to_string(),
        };
        tmpl_data.insert("k8s_deprecation_url".to_string(), to_json(deprecation_url));
        tmpl_data.insert(
            "k8s_release_url".to_string(),
            to_json(config_data.kubernetes.release_url),
        );

        tmpl_data.insert("custom_ami".to_string(), to_json(playbook.custom_ami));

        tmpl_data
    }
}

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

pub fn create(playbook: &Playbook) -> Result<(), anyhow::Error> {
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
