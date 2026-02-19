mod common;

use common::fixtures;

#[test]
fn mocks_compile() {
  let _aws = fixtures::healthy_aws();
  let _k8s = fixtures::healthy_k8s();
}
