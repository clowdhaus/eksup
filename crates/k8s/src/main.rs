mod cluster;
mod deprecated;

use kube::Client;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Loads that deprecated API data from local file
    let deprecated = deprecated::Deprecated::get()?;

    // Gets the APIs supported by the Kubernetes API server
    let client = Client::try_default().await?;
    let discovery = cluster::Discovery::get(&client).await?;

    // Checks if any of the deprecated APIs are still supported by the API server
    for (key, value) in &deprecated.versions {
        if discovery.versions.contains_key(key) {
            println!("DEPRECATED: {value:#?}");
        }
    }

    Ok(())
}
