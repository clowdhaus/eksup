#!/usr/bin/env bash

rm examples/*.md

for version in "1.20" "1.21" "1.22" "1.23" "1.24"; do
  cargo run -- create-playbook --cluster-version $version \
    --eks-managed-node-group \
    --filename "examples/eks-mng-${version}.md"

  cargo run -- create-playbook --cluster-version $version \
    --self-managed-node-group \
    --filename "examples/self-mng-${version}.md"

  cargo run -- create-playbook --cluster-version $version \
    --fargate-profile \
    --filename "examples/fargate-profile-${version}.md"

  cargo run -- create-playbook --cluster-version $version \
    --eks-managed-node-group \
    --self-managed-node-group \
    --fargate-profile \
    --filename "examples/all-${version}.md"
done
