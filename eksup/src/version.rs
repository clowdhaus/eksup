use anyhow::{Context, Result, bail};

/// Minimum supported minor version — clusters below this cannot be analyzed
pub const MINIMUM: i32 = 30;

/// Latest supported minor version
pub const LATEST: i32 = 35;

/// Format a minor version number as a full version string (e.g. 30 → "1.30")
pub fn format_version(minor: i32) -> String {
  format!("1.{minor}")
}

/// Early validation for CLI entry points.
/// Returns `Some(target_minor)` if upgrade is possible, `None` if already at latest.
/// Bails for below-minimum.
pub fn check_version_supported(cluster_version: &str) -> Result<Option<i32>> {
  let current_minor = parse_minor(cluster_version)?;
  if current_minor < MINIMUM {
    bail!(
      "Cluster version {cluster_version} is below the minimum supported version ({}). \
       Please upgrade to at least {} before using this tool.",
      format_version(MINIMUM),
      format_version(MINIMUM),
    );
  }
  if current_minor >= LATEST {
    return Ok(None);
  }
  Ok(Some(current_minor + 1))
}

/// Get the target minor version the cluster will be upgraded to
///
/// Given the current Kubernetes version and the default behavior based on Kubernetes
/// upgrade restrictions of one minor version upgrade at a time, return the
/// next minor version number
// Future: Support BlueGreen strategy where target can skip versions
pub fn get_target_version(current_version: &str) -> Result<i32> {
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

/// Validate a user-provided target version string against the cluster's current version.
///
/// Returns the target minor version number if valid.
/// Errors if the target is not parseable, not greater than the current version,
/// or exceeds `LATEST`.
pub fn validate_target_version(target: &str, cluster_version: &str) -> Result<i32> {
  let target_str = target.strip_prefix('v').unwrap_or(target);
  // Accept bare minor like "34" or full "1.34"
  let target_minor = if target_str.contains('.') {
    parse_minor(target_str)?
  } else {
    target_str.parse::<i32>()
      .context(format!("Invalid target version '{target}', expected format like '1.34' or '34'"))?
  };

  let current_minor = parse_minor(cluster_version)?;

  if target_minor <= current_minor {
    bail!(
      "Target version {} must be greater than current cluster version {}",
      format_version(target_minor),
      format_version(current_minor),
    );
  }
  if target_minor > LATEST {
    bail!(
      "Target version {} exceeds the latest supported version ({})",
      format_version(target_minor),
      format_version(LATEST),
    );
  }

  Ok(target_minor)
}

/// Given a version, parse the minor version
///
/// For example, the format Amazon EKS of v1.20.7-eks-123456 returns 20
/// Or the format of v1.22.7 returns 22
pub fn parse_minor(version: &str) -> Result<i32> {
  let parts: Vec<&str> = version.split('.').collect();
  let minor_str = parts.get(1)
    .context(format!("Invalid version format '{version}', expected 'X.Y[.Z]'"))?;
  let minor = minor_str.parse::<i32>()?;

  Ok(minor)
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

  #[test]
  fn check_version_supported_below_minimum() {
    let result = check_version_supported("1.29");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("below the minimum"), "error message: {msg}");
  }

  #[test]
  fn check_version_supported_at_latest() {
    let result = check_version_supported(&format_version(LATEST)).unwrap();
    assert!(result.is_none(), "should return None at latest");
  }

  #[test]
  fn check_version_supported_upgradeable() {
    let result = check_version_supported(&format_version(MINIMUM)).unwrap();
    assert_eq!(result, Some(MINIMUM + 1));
  }

  #[test]
  fn check_version_supported_one_below_latest() {
    let result = check_version_supported(&format_version(LATEST - 1)).unwrap();
    assert_eq!(result, Some(LATEST));
  }

  #[test]
  fn check_version_supported_above_latest() {
    let result = check_version_supported(&format_version(LATEST + 1)).unwrap();
    assert!(result.is_none(), "should return None above latest");
  }

  #[test]
  fn format_version_edge_cases() {
    assert_eq!(format_version(0), "1.0");
    assert_eq!(format_version(99), "1.99");
  }

  #[test]
  fn parse_minor_with_v_prefix() {
    assert_eq!(parse_minor("v1.30").unwrap(), 30);
    assert_eq!(parse_minor("v1.30.0-eks-12345").unwrap(), 30);
  }

  // ---- validate_target_version ----

  #[test]
  fn validate_target_version_single_hop() {
    let result = validate_target_version("1.31", "1.30").unwrap();
    assert_eq!(result, 31);
  }

  #[test]
  fn validate_target_version_multi_hop() {
    let result = validate_target_version("1.33", "1.30").unwrap();
    assert_eq!(result, 33);
  }

  #[test]
  fn validate_target_version_with_v_prefix() {
    let result = validate_target_version("v1.33", "1.30").unwrap();
    assert_eq!(result, 33);
  }

  #[test]
  fn validate_target_version_below_current() {
    let result = validate_target_version("1.29", "1.30");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("must be greater than"), "error: {msg}");
  }

  #[test]
  fn validate_target_version_equal_current() {
    let result = validate_target_version("1.30", "1.30");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("must be greater than"), "error: {msg}");
  }

  #[test]
  fn validate_target_version_above_latest() {
    let result = validate_target_version(&format_version(LATEST + 1), "1.30");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("exceeds the latest supported version"), "error: {msg}");
  }

  #[test]
  fn validate_target_version_at_latest() {
    // Target == LATEST is valid as long as current < LATEST
    let result = validate_target_version(&format_version(LATEST), &format_version(LATEST - 1)).unwrap();
    assert_eq!(result, LATEST);
  }
}
