// use k8s_openapi::api::core::v1::NodeSystemInfo;

// use crate::cli::KubernetesVersion;

// fn version_skew(
//   cp_version: KubernetesVersion,
//   nodes: &Vec<NodeSystemInfo>,
// ) -> Result<CheckResult, anyhow::Error> {
//   let mut result = CheckResult::new("version_skew");
//   let mut errors = Vec::new();

//   for node in nodes {
//     let node_version = KubernetesVersion::from_str(&node.kubelet_version)?;
//     if node_version.major != cp_version.major {
//       errors.push(format!(
//         "Node {} is running Kubernetes version {} but the control plane is running version {}",
//         node.name, node_version, cp_version
//       ));
//     }
//   }

//   if errors.is_empty() {
//     result.pass();
//   } else {
//     result.fail(errors);
//   }

//   Ok(result)
// }

fn _control_plane_ips() -> Result<Vec<String>, anyhow::Error> {
  todo!()
}
