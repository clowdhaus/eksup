use crate::aws;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OutputFormat {
  Json,
  Markdown,
  Stdout,
}

pub(crate) async fn version_skew(nodes: &[NodeDetail], otype: OutputFormat) -> Result<(), anyhow::Error> {

  Ok(())
}