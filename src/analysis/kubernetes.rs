// use k8s_openapi::api::core::v1::Node;
// use kube::api::{Api, ListParams};
use kube::{Client, Discovery};

pub async fn collect_from_nodes(client: Client) -> Result<(), anyhow::Error> {
    // let api_nodes: Api<Node> = Api::all(client);
    // let nodes = api_nodes.list(&ListParams::default()).await?;
    // extract_allocatable_from_nodes(nodes, resources).await?;
    // println!("{nodes:#?}");

    let discovery = Discovery::new(client.clone()).run().await?;
    resolve_api_resource(&discovery).await?;

    Ok(())
}

async fn resolve_api_resource(discovery: &Discovery) -> Result<(), anyhow::Error> {
    // iterate through groups to find matching kind/plural names at recommended versions
    // and then take the minimal match by group.name (equivalent to sorting groups by group.name).
    // this is equivalent to kubectl's api group preference
    let groups = discovery.groups();

    for group in groups {
        let name = group.name();
        let versions = group.versions();

        for version in versions {
            println!("{name}/{version}");
        }
    }
    Ok(())
}
