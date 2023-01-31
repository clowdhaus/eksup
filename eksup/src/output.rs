use std::{fs::File, io::prelude::*};

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::analysis;

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
pub enum Format {
  /// JSON format used for logging or writing to a *.json file
  Json,
  /// Text format used for writing to stdout
  Text,
}

impl Default for Format {
  fn default() -> Self {
    Self::Text
  }
}

pub(crate) async fn output(results: &analysis::Results, format: &Format, filename: &Option<String>) -> Result<()> {
  let output = match format {
    Format::Json => serde_json::to_string(&results)?,
    Format::Text => format!("{results:#?}"),
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
