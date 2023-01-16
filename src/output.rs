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

// TODO - replace this with standalone flags
// --stdout will output to stdout
// --filename <FILENAME> indicates writing to file under the defined name passed
// Need to write out all combination of commands and flags for intended use cases
// to see what makes the most sense and is the most intuitive/ergonomical
#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
pub enum OutputType {
  /// JSON format used for logging or writing to a *.json file
  Stdout,
  /// Write to file
  File,
}

impl Default for OutputType {
  fn default() -> Self {
    Self::Stdout
  }
}

pub(crate) async fn output(
  results: &analysis::Results,
  oformat: &OutputFormat,
  otype: &OutputType,
  filename: &str,
) -> Result<(), anyhow::Error> {
  let output = match oformat {
    OutputFormat::Json => serde_json::to_string(&results)?,
    OutputFormat::Text => format!("{results:#?}"),
  };

  match otype {
    OutputType::Stdout => {
      println!("{output}");
    }
    OutputType::File => {
      let mut file = File::create(filename)?;
      file.write_all(output.as_bytes())?;
    }
  }
  Ok(())
}
