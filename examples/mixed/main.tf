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
  name          = "test-${basename(path.cwd)}"
  minor_version = 27
  region        = "us-east-1"

  vpc_cidr_nodes = "10.0.0.0/16"
  vpc_cidr_pods  = "10.99.0.0/16"
  azs            = slice(data.aws_availability_zones.available.names, 0, 3)

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
  version = "~> 20.13"

  cluster_name                   = local.name
  cluster_version                = "1.${local.minor_version}"
  cluster_endpoint_public_access = true

  cluster_addons = {
    coredns = {
      # aws eks describe-addon-versions --kubernetes-version 1.27 --addon-name coredns --query 'addons[*].addonVersions[*].addonVersion'
      addon_version = "v1.9.3-eksbuild.3"
      configuration_values = jsonencode({
        computeType = "Fargate"
      })
    }
    kube-proxy = {
      # aws eks describe-addon-versions --kubernetes-version 1.27 --addon-name kube-proxy --query 'addons[*].addonVersions[*].addonVersion'
      addon_version = "v1.25.6-eksbuild.2"
    }
    vpc-cni = {
      # aws eks describe-addon-versions --kubernetes-version 1.27 --addon-name vpc-cni --query 'addons[*].addonVersions[*].addonVersion'
      addon_version  = "v1.12.5-eksbuild.2"
      before_compute = true
      configuration_values = jsonencode({
        env = {
          # Reference https://aws.github.io/aws-eks-best-practices/reliability/docs/networkmanagement/#cni-custom-networking
          # Reference https://docs.aws.amazon.com/eks/latest/userguide/cni-increase-ip-addresses.html
          AWS_VPC_K8S_CNI_CUSTOM_NETWORK_CFG = "true"
          ENI_CONFIG_LABEL_DEF               = "failure-domain.beta.kubernetes.io/zone"
        }
      })
    }
  }

  vpc_id = module.vpc.vpc_id
  # We only want to assign the 10.0.* range subnets to the data plane
  subnet_ids               = slice(module.vpc.private_subnets, 0, 3)
  control_plane_subnet_ids = module.vpc.intra_subnets

  eks_managed_node_group_defaults = {
    # Demonstrating skew check
    cluster_version = "1.${local.minor_version - 1}"
  }

  eks_managed_node_groups = {
    # This uses a custom launch template (custom as in module/user supplied)
    standard = {
      # pre_bootstrap_user_data = <<-EOT
      #   #!/bin/bash
      #   echo "Hello from user data!"
      # EOT

      # To show pending changes
      update_launch_template_default_version = false

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
    cluster_version = "1.${local.minor_version - 2}"
  }

  self_managed_node_groups = {
    standard = {
      min_size     = 1
      max_size     = 3
      desired_size = 1
    }

    different = {
      # pre_bootstrap_user_data = <<-EOT
      #   #!/bin/bash
      #   echo "Hello from user data!"
      # EOT

      # To show pending changes
      instance_refresh                       = {}
      update_launch_template_default_version = false
    }
  }

  fargate_profiles = {
    kube_system = {
      name = "kube-system"
      selectors = [
        { namespace = "kube-system" }
      ]
    }
  }

  tags = local.tags
}

################################################################################
# VPC-CNI Custom Networking ENIConfig
################################################################################

resource "kubectl_manifest" "eni_config" {
  for_each = zipmap(local.azs, slice(module.vpc.private_subnets, 3, 6))

  yaml_body = yamlencode({
    apiVersion = "crd.k8s.amazonaws.com/v1alpha1"
    kind       = "ENIConfig"
    metadata = {
      name = each.key
    }
    spec = {
      securityGroups = [
        module.eks.cluster_primary_security_group_id,
        module.eks.node_security_group_id,
      ]
      subnet = each.value
    }
  })
}
################################################################################
# Supporting Resources
################################################################################

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = local.name
  cidr = local.vpc_cidr_nodes

  secondary_cidr_blocks = [local.vpc_cidr_pods] # can add up to 5 total CIDR blocks

  azs = local.azs
  private_subnets = concat(
    [for k, v in local.azs : cidrsubnet(local.vpc_cidr_nodes, 4, k)],
    [for k, v in local.azs : cidrsubnet(local.vpc_cidr_pods, 2, k)]
  )
  public_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr_nodes, 8, k + 48)]
  intra_subnets  = [for k, v in local.azs : cidrsubnet(local.vpc_cidr_nodes, 8, k + 52)]

  enable_nat_gateway = true
  single_nat_gateway = true

  public_subnet_tags = {
    "kubernetes.io/role/elb" = 1
  }

  private_subnet_tags = {
    "kubernetes.io/role/internal-elb" = 1
  }

  tags = local.tags
}
