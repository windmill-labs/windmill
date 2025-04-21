# windmill-cloudformation
Cloudformation Template for Windmill on AWS EKS

## Overview

This CloudFormation template automatically deploys Windmill on AWS EKS. The deployment includes:

- An EKS cluster with configurable node types and sizes
- An RDS PostgreSQL database for Windmill data
- AWS Load Balancer Controller for handling ingress traffic
- Proper network configuration with VPC, subnets, and security groups
- A fully automated installation of Windmill via Helm

## Parameters

The template accepts various parameters to customize your deployment:

- **NodeInstanceType**: EC2 instance type for EKS worker nodes (t3.small to r5.2xlarge)
- **NodeGroupSize**: Number of EKS worker nodes
- **RdsInstanceClass**: RDS instance class for the PostgreSQL database (db.t3.micro to db.r5.2xlarge)
- **DBPassword**: Password for the PostgreSQL database
- **WorkerReplicas**: Number of Windmill worker replicas
- **NativeWorkerReplicas**: Number of Windmill native worker replicas
- **Enterprise**: Enable Windmill [Enterprise features](https://www.windmill.dev/docs/misc/plans_details#upgrading-to-enterprise-edition) (requires license key)

## Customization

To modify the Helm chart configuration or update the template, refer to the official Windmill Helm chart repository:
[https://github.com/windmill-labs/windmill-helm-charts](https://github.com/windmill-labs/windmill-helm-charts)

## Documentation

For more information about Windmill's Helm chart deployment options, see:
[https://www.windmill.dev/docs/advanced/self_host#helm-chart](https://www.windmill.dev/docs/advanced/self_host#helm-chart)

For detailed information about setting up RDS for Windmill on AWS:
[https://www.windmill.dev/docs/advanced/self_host/aws_ecs#create-a-rds-database](https://www.windmill.dev/docs/advanced/self_host/aws_ecs#create-a-rds-database)

## Deployment

1. Upload the CloudFormation template to your AWS account
2. Fill in the required parameters
3. Deploy the stack
4. Access Windmill using the URL provided in the Outputs section of the stack

After deployment, you can access Windmill via the LoadBalancer URL shown in the CloudFormation stack outputs.
