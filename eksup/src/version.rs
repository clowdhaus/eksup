use anyhow::{Context, Result, bail};

/// Minimum supported minor version — clusters below this cannot be analyzed
pub const MINIMUM: i32 = 30;

/// Latest supported minor version
pub const LATEST: i32 = 35;

/// Format a minor version number as a full version string (e.g. 30 → "1.30")
pub(crate) fn format_version(minor: i32) -> String {
  format!("1.{minor}")
}

/// Get the target minor version the cluster will be upgraded to
///
/// Given the current Kubernetes version and the default behavior based on Kubernetes
/// upgrade restrictions of one minor version upgrade at a time, return the
/// next minor version number
/// TODO: This will change in the future when the strategy allows for `BlueGreen` upgrades
pub(crate) fn get_target_version(current_version: &str) -> Result<i32> {
  let current_minor = parse_minor(current_version)?;

  if current_minor < MINIMUM {
    bail!(
      "Cluster version {current_version} is below the minimum supported version ({}). \
       Please upgrade to at least {} before using this tool.",
      format_version(MINIMUM),
      format_version(MINIMUM),
    );
  }
  if current_minor >= LATEST {
    bail!(
      "Cluster is already on the latest supported version ({})",
      format_version(LATEST),
    );
  }
  Ok(current_minor + 1)
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
  fn parse_minor_valid_versions() {
    let cases = vec![
      ("v1.20.7-eks-123456", 20),
      ("1.30", 30),
      ("v1.30", 30),
      ("v1.30.0-eks-12345", 30),
      ("1.25.3", 25),
    ];

    for (input, expected) in cases {
      let result = parse_minor(input).unwrap();
      assert_eq!(result, expected, "parse_minor({input})");
    }
  }

  #[test]
  fn parse_minor_invalid_versions() {
    assert!(parse_minor("125").is_err(), "should fail on '125' (no dot)");
    assert!(parse_minor("").is_err(), "should fail on empty string");
  }

  #[test]
  fn normalize_valid_versions() {
    let cases = vec![
      ("v1.30.0-eks-12345", "1.30"),
      ("1.25", "1.25"),
      ("v1.20.7-eks-123456", "1.20"),
    ];

    for (input, expected) in cases {
      let result = normalize(input).unwrap();
      assert_eq!(result, expected, "normalize({input})");
    }
  }

  #[test]
  fn normalize_invalid_versions() {
    assert!(normalize("nodots").is_err(), "should fail on 'nodots'");
  }

  #[test]
  fn format_version_formats_correctly() {
    assert_eq!(format_version(30), "1.30");
    assert_eq!(format_version(35), "1.35");
  }

  #[test]
  fn get_target_version_increments_minor() {
    let result = get_target_version(&format_version(MINIMUM)).unwrap();
    assert_eq!(result, MINIMUM + 1);
  }

  #[test]
  fn get_target_version_errors_on_latest() {
    let result = get_target_version(&format_version(LATEST));
    assert!(result.is_err(), "should error when already on LATEST");
  }

  #[test]
  fn get_target_version_errors_below_minimum() {
    let result = get_target_version("1.29");
    assert!(result.is_err(), "should error when below MINIMUM");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("below the minimum supported version"), "error message: {msg}");
  }

  #[test]
  fn get_target_version_at_minimum() {
    let result = get_target_version(&format_version(MINIMUM));
    assert!(result.is_ok(), "should succeed at MINIMUM");
  }
}
