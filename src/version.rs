/// Given a version, parse the minor version
///
/// For example, the format Amazon EKS of v1.20.7-eks-123456 returns 20
/// Or the format of v1.22.7 returns 22
pub(crate) fn parse_minor(version: &str) -> Result<u32, anyhow::Error> {
  let version = version.split('.').collect::<Vec<&str>>();
  let minor = version[1].parse::<u32>()?;

  Ok(minor)
}

/// Given a version, normalize to a consistent format
///
/// For example, the format Amazon EKS uses is v1.20.7-eks-123456 which is normalized to 1.20
pub(crate) fn normalize(version: &str) -> Result<String, anyhow::Error> {
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
