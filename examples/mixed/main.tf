provider "aws" {
  region = local.region
}

provider "kubernetes" {
  host                   = module.eks.cluster_endpoint
  cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

  exec {
    api_version = "client.authentication.k8s.io/v1beta1"
    command     = "aws"
    # This requires the awscli to be installed locally where Terraform is executed
    args = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
  }
}

data "aws_caller_identity" "current" {}
data "aws_availability_zones" "available" {}

locals {
  name                  = "test-${basename(path.cwd)}"
  control_plane_version = "1.21" # Update after deploy to create skew
  data_plane_version    = "1.21"
  region                = "us-east-1"

  vpc_cidr = "10.0.0.0/16"
  azs      = slice(data.aws_availability_zones.available.names, 0, 3)

  tags = {
    Example    = local.name
    GithubRepo = "github.com/clowdhaus/eksup"
  }
}

################################################################################
# EKS Module
################################################################################

module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 19.5"

  cluster_name                   = local.name
  cluster_version                = local.control_plane_version
  cluster_endpoint_public_access = true

  cluster_addons = {
    coredns = {
      # aws eks describe-addon-versions --kubernetes-version 1.21 --addon-name coredns
      addon_version = "v1.8.3-eksbuild.1"
      configuration_values = jsonencode({
        computeType = "Fargate"
      })
    }
    kube-proxy = {
      # aws eks describe-addon-versions --kubernetes-version 1.21 --addon-name kube-proxy
      addon_version = "v1.19.6-eksbuild.2"
    }
    vpc-cni = {
      # aws eks describe-addon-versions --kubernetes-version 1.21 --addon-name vpc-cni
      addon_version = "v1.6.3-eksbuild.2"
    }
  }

  vpc_id                   = module.vpc.vpc_id
  subnet_ids               = module.vpc.private_subnets
  control_plane_subnet_ids = module.vpc.intra_subnets

  # Required to register the self-managed node groups with the cluster
  manage_aws_auth_configmap = true

  eks_managed_node_group_defaults = {
    # Demonstrating skew check
    cluster_version = local.data_plane_version
  }

  eks_managed_node_groups = {
    # This uses a custom launch template (custom as in module/user supplied)
    standard = {
      # pre_bootstrap_user_data = <<-EOT
      #   #!/bin/bash
      #   echo "Hello from user data!"
      # EOT

      min_size     = 1
      max_size     = 3
      desired_size = 1
    }

    # This uses the default launch template created by EKS MNG
    default = {
      use_custom_launch_template = false
    }
  }

  self_managed_node_group_defaults = {
    # Demonstrating skew check
    cluster_version = local.data_plane_version
  }

  self_managed_node_groups = {
    standard = {
      min_size     = 1
      max_size     = 3
      desired_size = 1
    }
  }


  fargate_profiles = merge(
    { for i in range(3) :
      "kube-system-${element(split("-", local.azs[i]), 2)}" => {
        selectors = [
          { namespace = "kube-system" }
        ]
        # We want to create a profile per AZ for high availability
        subnet_ids = [element(module.vpc.private_subnets, i)]
      }
    },
  )

  tags = local.tags
}

################################################################################
# Supporting Resources
################################################################################

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 3.0"

  name = local.name
  cidr = local.vpc_cidr

  azs             = local.azs
  private_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 4, k)]
  public_subnets  = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 48)]
  intra_subnets   = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 52)]

  enable_nat_gateway   = true
  single_nat_gateway   = true
  enable_dns_hostnames = true

  enable_flow_log                      = true
  create_flow_log_cloudwatch_iam_role  = true
  create_flow_log_cloudwatch_log_group = true

  public_subnet_tags = {
    "kubernetes.io/role/elb" = 1
  }

  private_subnet_tags = {
    "kubernetes.io/role/internal-elb" = 1
  }

  tags = local.tags
}
