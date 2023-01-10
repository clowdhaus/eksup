# AWS

## Control Plane

- Version
  - [x] [CHECK] Version skew between control plane and data plane
- Subnets
  - [x] [CHECK] Number of free IPs > 5

## Data Plane

- Subnets
  - [x] [CHECK] Percentage of total IPs available
- EKS managed node group(s)
  - [x] [CHECK] Node group health
  - [ ] [CHECK] Pending updates (launch template version to be deployed)
- Self-managed node group(s)
  - [ ] [CHECK] Pending updates (launch template version to be deployed)
- Fargate Profile(s)

## Addons

- Version(s)
  - [ ] [CHECK] Pending updates
  - [ ] [CHECK] Default version for target Kubernetes version

## Misc

- Service limits
  - Requires premium support https://docs.aws.amazon.com/awssupport/latest/user/service-limits.html
  - [ ] [CHECK] EC2 instance service limits
    - `aws support describe-trusted-advisor-check-result --check-id 0Xc6LMYG8P`
  - [ ] [CHECK] EBS volume service limits
    - `aws support describe-trusted-advisor-check-result --check-id dH7RR0l6J9` GP2
    - `aws support describe-trusted-advisor-check-result --check-id dH7RR0l6J3` GP3
