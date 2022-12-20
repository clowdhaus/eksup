#!/usr/bin/env bash

for version in "1.20" "1.21" "1.22" "1.23" "1.24"; do
  cargo run -- --cluster-version $version \
    --eks-managed-node-group \
    --self-managed-node-group \
    --fargate-profile \
    --filename "examples/standard-ami-${version}.md"

  cargo run -- --cluster-version $version \
    --eks-managed-node-group \
    --self-managed-node-group \
    --fargate-profile \
    --custom-ami \
    --filename "examples/custom-ami-${version}.md"
done
