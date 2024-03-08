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
