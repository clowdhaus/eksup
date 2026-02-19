use anyhow::{Context, Result};

/// Latest support version
pub const LATEST: &str = "1.35";

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
  let parts: Vec<&str> = version.split('.').collect();
  let minor_str = parts.get(1)
    .context(format!("Invalid version format '{version}', expected 'X.Y[.Z]'"))?;
  let minor = minor_str.parse::<i32>()?;

  Ok(minor)
}

/// Given a version, normalize to a consistent format
///
/// For example, the format Amazon EKS uses is v1.20.7-eks-123456 which is normalized to 1.20
pub(crate) fn normalize(version: &str) -> Result<String> {
  let parts: Vec<&str> = version.split('.').collect();
  let major = parts.first()
    .context(format!("Invalid version format '{version}'"))?;
  let minor = parts.get(1)
    .context(format!("Invalid version format '{version}', expected 'X.Y[.Z]'"))?;

  Ok(format!("{}.{}", major.replace('v', ""), minor))
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
