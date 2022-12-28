use config::{Config, ConfigError, File};
use kube::{Client, Discovery};
use serde::Deserialize;

// A version holds the information about a deprecated API version and potential replacement API
#[derive(Deserialize, Debug)]
struct Version {
    // Name of the API version
    api_version: String,
    // Kind of the object associated with this version
    kind: String,
    // DeprecatedIn indicates what version the API is deprecated in
    // an empty string indicates that the version is not deprecated
    deprecated_in: Option<String>,
    // RemovedIn denotes the version that the api was actually removed in
    // `None` indicates that the version has not been removed yet
    removed_in: Option<String>,
    // ReplacementAPI is the apiVersion that replaces the deprecated one
    replacement_api_version: Option<String>,
}

// Holds the deprecated API versions
#[derive(Deserialize, Debug)]
struct Deprecations {
    deprecations: Vec<Version>,
}

impl Deprecations {
    pub fn new(filename: String) -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name(&filename))
            .build()?;

        s.try_deserialize()
    }
}


// async fn _get_client() -> Result<Client, anyhow::Error> {
//     let client = Client::try_default().await?;
//     Ok(client)
// }

pub async fn collect_from_nodes(_client: Client) -> Result<(), anyhow::Error> {
    let deprecations = Deprecations::new("src/analysis/kubernetes.yaml".to_string());

    println!("{deprecations:#?}");

    // let discovery = Discovery::new(client.clone()).run().await?;
    // resolve_api_resource(&discovery).await?;

    Ok(())
}

// async fn _resolve_api_resource(discovery: &Discovery) -> Result<(), anyhow::Error> {
//     // iterate through groups to find matching kind/plural names at recommended versions
//     // and then take the minimal match by group.name (equivalent to sorting groups by group.name).
//     // this is equivalent to kubectl's api group preference
//     let groups = discovery.groups();

//     for group in groups {
//         let _name = group.name();
//         let _versions = group.versions();

//         // for version in versions {
//         //     println!("{name}/{version}");
//         // }
//     }
//     Ok(())
// }
