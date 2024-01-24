use std::fmt;

use anyhow::Result;
use clap::ValueEnum;
use seq_macro::seq;
use serde::{Deserialize, Serialize};

/// Latest support version
pub const LATEST: &str = "1.29";

#[derive(Debug, Serialize, Deserialize)]
pub struct Versions {
  pub current: String,
  pub target: String,
}

seq!(N in 23..=29 {
    /// Kubernetes version(s) supported
    #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
    pub enum KubernetesVersion {
        #( V~N, )*
    }

    /// Formats the Kubernetes version as a string in the form of "1.X"
    impl fmt::Display for KubernetesVersion {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                #( KubernetesVersion::V~N => write!(f, "1.{}", N), )*
            }
        }
    }

    /// Used by clap for acceptable values and converting from input to enum
    impl ValueEnum for KubernetesVersion {
        fn value_variants<'a>() -> &'a [Self] {
            &[
                #( Self::V~N, )*
            ]
        }

        fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
            match self {
                #( Self::V~N => Some(clap::builder::PossibleValue::new(format!("1.{}", N))), )*
            }
        }
    }
});

/// Get the Kubernetes version the cluster is intended to be upgraded to
///
/// Given the current Kubernetes version and the default behavior based on Kubernetes
/// upgrade restrictions of one minor version upgrade at a time, return the
/// next minor Kubernetes version
/// TODO: This will change in the future when the strategy allows for `BlueGreen` upgrades
pub(crate) fn get_target_version(current_version: &str) -> Result<String> {
  let current_minor_version = current_version.split('.').collect::<Vec<&str>>()[1].parse::<i32>()?;

  Ok(format!("1.{}", current_minor_version + 1))
}

/// Given a version, parse the minor version
///
/// For example, the format Amazon EKS of v1.20.7-eks-123456 returns 20
/// Or the format of v1.22.7 returns 22
pub(crate) fn parse_minor(version: &str) -> Result<i32> {
  let version = version.split('.').collect::<Vec<&str>>();
  let minor = version[1].parse::<i32>()?;

  Ok(minor)
}

/// Given a version, normalize to a consistent format
///
/// For example, the format Amazon EKS uses is v1.20.7-eks-123456 which is normalized to 1.20
pub(crate) fn normalize(version: &str) -> Result<String> {
  let version = version.split('.').collect::<Vec<&str>>();
  let normalized = format!("{}.{}", version[0].replace('v', ""), version[1]);

  Ok(normalized)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn can_parse_minor() {
    let input_expected = vec![
      ("v1.20.7-eks-123456", 20),
      // TODO - add more test cases and failure cases
    ];

    for (input, expected) in input_expected {
      let result = parse_minor(input).unwrap();
      assert_eq!(result, expected);
    }
  }

  #[test]
  fn can_normalize() {
    let input_expected = vec![
      ("v1.20.7-eks-123456", "1.20"),
      // TODO - add more test cases and failure cases
    ];

    for (input, expected) in input_expected {
      let result = normalize(input).unwrap();
      assert_eq!(result, expected);
    }
  }
}
