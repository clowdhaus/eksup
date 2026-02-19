use std::{fs::File, io::prelude::*};

use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::analysis;

/// Converts vec into comma separated string for tabled output
pub fn tabled_vec_to_string(v: &[String]) -> String {
  v.join(", ")
}

#[derive(Clone, Copy, Debug, Default, ValueEnum, Serialize, Deserialize)]
pub enum Format {
  /// JSON format used for logging or writing to a *.json file
  Json,
  /// Text format used for writing to stdout
  #[default]
  Text,
}

pub(crate) fn output(results: &analysis::Results, format: &Format, filename: &Option<String>) -> Result<()> {
  let output = match format {
    Format::Json => serde_json::to_string_pretty(&results)?,
    Format::Text => results.to_stdout_table()?,
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
