use std::{fs::File, io::prelude::*};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::analysis;

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
pub enum OutputFormat {
  /// JSON format used for logging or writing to a *.json file
  Json,
  /// Text format used for writing to stdout
  Text,
}

impl Default for OutputFormat {
  fn default() -> Self {
    Self::Text
  }
}

pub(crate) async fn output(
  findings: &analysis::Findings,
  format: &OutputFormat,
  filename: &Option<String>,
) -> Result<(), anyhow::Error> {
  let output = match format {
    OutputFormat::Json => serde_json::to_string(&findings)?,
    OutputFormat::Text => format!("{findings:#?}"),
  };

  match filename {
    Some(filename) => {
      let mut file = File::create(filename)?;
      file.write_all(output.as_bytes())?;
    }
    None => {
      println!("{output}");
    }
  }

  Ok(())
}
