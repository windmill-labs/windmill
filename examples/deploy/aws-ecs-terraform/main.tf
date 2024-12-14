terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.16"
    }
  }

  required_version = ">= 1.2.0"
}

provider "aws" {
  region  = "us-east-2"
  profile = "terraform"
}

data "aws_region" "current" {}

data "aws_iam_role" "ecs_task_execution_role" {
  # this needs to be creates in AWS with the attaching the AWS managed policy: 'AmazonECSTaskExecutionRolePolicy'
  name = "ecsTaskExecutionRole"
}

variable "database_password" {
  type        = string
  sensitive   = true
  description = "RDS Database password"
}

locals {
  db_url = "postgres://${aws_db_instance.windmill_cluster_rds.username}:${aws_db_instance.windmill_cluster_rds.password}@${aws_db_instance.windmill_cluster_rds.endpoint}/${aws_db_instance.windmill_cluster_rds.db_name}"
}

data "aws_ssm_parameter" "amazon_linux_2023" {
  name = "/aws/service/ecs/optimized-ami/amazon-linux-2023/recommended/image_id"
}
