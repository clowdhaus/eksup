#!/usr/bin/env bash

rm examples/*.md

for version in "1.20" "1.21" "1.22" "1.23" "1.24"; do
  cargo run -- create-playbook --cluster-version $version \
    --compute eks \
    --filename "examples/eks-mng-${version}.md"

  cargo run -- create-playbook --cluster-version $version \
    --compute self \
    --filename "examples/self-mng-${version}.md"

  cargo run -- create-playbook --cluster-version $version \
    --compute fargate \
    --filename "examples/fargate-profile-${version}.md"

  cargo run -- create-playbook --cluster-version $version \
    --compute eks self fargate \
    --filename "examples/all-${version}.md"
done
