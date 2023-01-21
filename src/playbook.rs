use std::{collections::HashMap, fs};

use handlebars::Handlebars;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

use crate::{cli::Playbook, version};

/// Embeds the contents of the `templates/` directory into the binary
///
/// This struct contains both the templates used for rendering the playbook
/// as well as the static data used for populating the playbook templates
/// embedded into the binary for distribution
#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

/// Relevant data for a Kubernetes release
///
/// Used to populate the playbook templates with the data associated
/// to a specific Kubernetes release version
#[derive(Serialize, Deserialize, Debug)]
struct Release {
  release_url: String,
  deprecation_url: Option<String>,
}

/// Type alias for Kubernetes version string (i.e. - "1.21")
type Version = String;

/// Data to populate the template(s) for rendering the upgrade playbook
///
/// This combines the static data from the `data.yaml` embedded along with
/// data collected from CLI arguments provided by users and is used to
/// populate the playbook templates when rendered. This also serves as
/// the central authority for the data/inputs used to populate the playbook
#[derive(Serialize, Deserialize, Debug)]
pub struct TemplateData {
  ///
  cluster_name: String,

  current_version: String,

  target_version: String,

  k8s_release_url: String,

  k8s_deprecation_url: String,

  eks_managed_node_group: Option<String>,

  self_managed_node_group: Option<String>,

  fargate_profile: Option<String>,
}

/// Load configuration data from the associated Kubernetes version data file
#[allow(unused)]
impl TemplateData {
  fn new(playbook: &Playbook) -> Result<Self, anyhow::Error> {
    let data_file = Templates::get("data.yaml").unwrap();
    let contents = std::str::from_utf8(data_file.data.as_ref())?;
    let data: HashMap<Version, Release> = serde_yaml::from_str(contents)?;

    let cluster_name = "foo".to_string(); // playbook.cluster_name.as_ref().unwrap();
    let current_version = "1.22".to_string(); // playbook.cluster_version.to_string();
    let target_version = version::get_target_version("1.22")?; // version::get_target_version(&current_version)?;
    let release = data.get(&target_version).unwrap();

    Ok(TemplateData {
      cluster_name,
      current_version,
      target_version,
      k8s_release_url: release.release_url.to_string(),
      k8s_deprecation_url: match &release.deprecation_url {
        Some(url) => url.to_string(),
        None => "".to_string(),
      },
      // TODO: Should this be a separate data structure since we are mutating
      // it after the fact? Plus, these are templates that are rendered with
      // the same data passed to the playbook template (in this very struct)
      eks_managed_node_group: None,
      self_managed_node_group: None,
      fargate_profile: None,
    })
  }
}

pub fn create(playbook: &Playbook) -> Result<(), anyhow::Error> {
  let mut handlebars = Handlebars::new();
  handlebars.register_embed_templates::<Templates>()?;

  let tmpl_data = TemplateData::new(playbook)?;

  // // Render sub-templates for data plane components
  // let eks_managed_node_group = if playbook.compute.contains(&Compute::EksManaged) {
  //   let rendered = handlebars.render("eks-managed-node-group.md", &tmpl_data)?;
  //   Some(rendered)
  // } else {
  //   None
  // };
  // tmpl_data.eks_managed_node_group = eks_managed_node_group;

  // let self_managed_node_group = if playbook.compute.contains(&Compute::SelfManaged) {
  //   let rendered = handlebars.render("self-managed-node-group.md", &tmpl_data)?;
  //   Some(rendered)
  // } else {
  //   None
  // };
  // tmpl_data.self_managed_node_group = self_managed_node_group;

  // let fargate_profile = if playbook.compute.contains(&Compute::FargateProfile) {
  //   let rendered = handlebars.render("fargate-profile.md", &tmpl_data)?;
  //   Some(rendered)
  // } else {
  //   None
  // };
  // tmpl_data.fargate_profile = fargate_profile;

  let filename = match &playbook.filename {
    Some(filename) => filename,
    // TODO - update default name to include cluster name, versions, etc. that would make it unique
    None => "playbook.md",
  };

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
  fs::write(filename, replaced)?;

  Ok(())
}
