use config::{Config, ConfigError, File};
use handlebars::to_json;
use serde::Deserialize;
use serde_json::value::{Map, Value as Json};

use crate::Upgrade;

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

    pub fn get_data(upgrade: Upgrade) -> Map<String, Json> {
        let mut tmpl_data = Map::new();

        let version = upgrade.cluster_version.to_string();

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

        tmpl_data.insert("custom_ami".to_string(), to_json(upgrade.custom_ami));

        tmpl_data
    }
}
