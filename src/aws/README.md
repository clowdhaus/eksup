# AWS

## Control Plane

- Version
  - [CHECK] Control plane version == data plane version
- Subnets
  - [CHECK] Number of free IPs > 5
- IP version (IPv4 or IPv6)

## Data Plane

- Subnets
  - [CHECK] Percentage of total IPs available
- EKS managed node group(s)
  - [CHECK] Pending updates (launch template version to be deployed)
  - [CHECK] AMI is custom or EKS optimized
- Self-managed node group(s)
  - [CHECK] Pending updates (launch template version to be deployed)
  - [CHECK] AMI is custom or EKS optimized
- Fargate Profile(s)

## Addons

- Version(s)
  - [CHECK] Pending updates
  - [CHECK] Default version for target Kubernetes version

## Misc

- Service limits
  - [CHECK] EC2 instance service limits
  - [CHECK] EBS volume service limits
