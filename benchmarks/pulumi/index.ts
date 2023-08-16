import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as tailscale from "@pulumi/tailscale";
import { randomInt } from "crypto";

const vpc = new aws.ec2.Vpc("bench", { cidrBlock: "10.0.0.0/16" });

// Create a security group allowing inbound access over port 80 and outbound
// access to anywhere.
const securityGroup = new aws.ec2.SecurityGroup("bench", {
  vpcId: vpc.id,
  ingress: [
    {
      cidrBlocks: ["0.0.0.0/0"],
      protocol: "tcp",
      fromPort: 80,
      toPort: 80,
    },
    {
      cidrBlocks: ["0.0.0.0/0"],
      protocol: "tcp",
      fromPort: 22,
      toPort: 22,
    },
  ],
  egress: [
    {
      cidrBlocks: ["0.0.0.0/0"],
      fromPort: 0,
      toPort: 0,
      protocol: "-1",
    },
  ],
});

// Create an an internet gateway.
const gateway = new aws.ec2.InternetGateway("gw", {
  vpcId: vpc.id,
});

// Find the latest Amazon Linux 2 AMI.
const ami = pulumi.output(
  aws.ec2.getAmi({
    owners: ["amazon"],
    mostRecent: true,
    filters: [{ name: "description", values: ["Amazon Linux 2 *"] }],
  })
);

// Create a route table.
const routes = new aws.ec2.RouteTable("route", {
  vpcId: vpc.id,
  routes: [
    {
      cidrBlock: "0.0.0.0/0",
      gatewayId: gateway.id,
    },
  ],
});

const subnetA = new aws.ec2.Subnet("bench-a", {
  availabilityZone: "us-east-2a",
  vpcId: vpc.id,
  cidrBlock: "10.0.5.0/24",
  mapPublicIpOnLaunch: true,
  tags: {
    Name: "bench",
  },
});

// Associate the route table with the public subnet.
const routeTableAssociation = new aws.ec2.RouteTableAssociation("bench-a", {
  subnetId: subnetA.id,
  routeTableId: routes.id,
});

const subnetB = new aws.ec2.Subnet("bench-b", {
  availabilityZone: "us-east-2b",
  vpcId: vpc.id,
  cidrBlock: "10.0.6.0/24",
  tags: {
    Name: "bench",
  },
});

const rdSubnet = new aws.rds.SubnetGroup("bench", {
  subnetIds: [subnetA.id, subnetB.id],
  tags: {
    Name: "bench",
  },
});

const db = new aws.rds.Instance("bench", {
  vpcSecurityGroupIds: [securityGroup.id],
  dbSubnetGroupName: rdSubnet.name,
  allocatedStorage: 20,
  dbName: "windmill",
  engine: "postgres",
  engineVersion: "14.8",
  instanceClass: "db.t3.micro",
  password: "postgres",
  skipFinalSnapshot: true,
  username: "postgres",
  availabilityZone: "us-east-2a",
});

const deployer = new aws.ec2.KeyPair("bench", {
  publicKey:
    "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQDRi2xytvAIgfs4f5B8EVrWH3bsu8CI+98iBazq08TuPEkLFP/ZqIVm8ZoTnpPadj2o0DXWHNjiyDAnGwp6W1rcSou4KCxoBViRX0doXBMat5GWs6Za2c+bmKllWUH2jhy4aPrdoI3il2URi7dIMH6WpWZDtD98CFK22rTgiBxhDydFpn3X1ULOI18f/SZEKzcb+iffhP5K/Sjqo65xPiKSl2m40vtaQjn2jTE7vmTD9c/dLv/5A3Sgh7CW2PJe9HR7/8qkzZkRkXtyODOIFfWN+vk0rGvk97IpV+DlSUag+FFeSK5Jn41w9IQV7frPvJAQOZ6eLq2GNv8iO/7QnfhlIUlPeClNJBRgb76FmiuslH+AOBiy/Szy3KUtoV/LO+uzvJxRZVX9DhSvTCm4D2qUTjozS+OSeBVuug70euWe29I7djouAuIbDOFkIkxStjhwISXfgvyWYVh8IMxC6+umZMhMff5CfqzpHY0jy2Tw5r/v/2Iz8EQXNRNAxnqKMPE= rfiszel@rubx1",
});

const sampleKey = new tailscale.TailnetKey("bench-ipn", {
  ephemeral: false,
  expiry: 60 * 60 * 24 * 90,
  preauthorized: true,
  reusable: true,
});

const ipn = new aws.ec2.Instance("bench-ipn", {
  ami: ami.id,
  availabilityZone: "us-east-2a",
  keyName: deployer.keyName,
  instanceType: "t3.medium",
  subnetId: subnetA.id,
  associatePublicIpAddress: true,
  rootBlockDevice: {
    volumeSize: 40,
  },
  vpcSecurityGroupIds: pulumi
    .all([db.vpcSecurityGroupIds, securityGroup.id])
    .apply(([db, sg]) => db.concat(sg)),
  userData: pulumi.interpolate`#!/bin/bash
sudo yum update -y
sudo yum install -y yum-utils
sudo yum-config-manager --add-repo https://pkgs.tailscale.com/stable/amazon-linux/2/tailscale.repo
sudo yum install -y tailscale
sudo systemctl enable --now tailscaled
sleep 30
sudo tailscale up --authkey="${sampleKey.key}" --hostname=bench-ipn --ssh --advertise-tags=tag:ipns

sudo yum install -y docker
sudo service docker start
sudo systemctl enable docker
sudo systemctl start docker

sudo curl https://raw.githubusercontent.com/windmill-labs/windmill/main/docker-compose.yml -o docker-compose.yml
sudo curl https://raw.githubusercontent.com/windmill-labs/windmill/main/Caddyfile -o Caddyfile
sudo curl https://raw.githubusercontent.com/windmill-labs/windmill/main/.env -o .env
sudo curl https://raw.githubusercontent.com/windmill-labs/windmill/main/oauth.json -o oauth.json


sudo yum install -y wget
wget https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m) 
sudo mv docker-compose-$(uname -s)-$(uname -m) /usr/local/bin/docker-compose
sudo chmod -v +x /usr/local/bin/docker-compose

sudo docker-compose up -d
`,
});

// Find the latest Amazon Linux 2 AMI.
const amiEcs = pulumi.output(
  aws.ec2.getAmi({
    owners: ["amazon"],
    mostRecent: true,
    filters: [{ name: "name", values: ["amzn2-ami-ecs-*-x86_64-*"] }],
  })
);
const lt = new aws.ec2.LaunchTemplate("asg", {
  namePrefix: "windmill",
  imageId: amiEcs.id,
  keyName: deployer.keyName,
  instanceType: "t3.medium",
  iamInstanceProfile: {
    name: "ecsInstanceRole",
  },
  updateDefaultVersion: true,
  vpcSecurityGroupIds: [securityGroup.id],
});

const asg = new aws.autoscaling.Group("asg", {
  availabilityZones: ["us-east-2a"],
  launchTemplate: {
    id: lt.id,
    version: "$Latest",
  },

  minSize: 0,
  maxSize: 1,
  tags: [
    {
      key: "AmazonECSManaged",
      value: "true",
      propagateAtLaunch: true,
    },
  ],
});

const capProvider = new aws.ecs.CapacityProvider("cap-provider", {
  autoScalingGroupProvider: {
    autoScalingGroupArn: asg.arn,
    managedTerminationProtection: "DISABLED",
    managedScaling: {
      maximumScalingStepSize: 1000,
      minimumScalingStepSize: 1,
      status: "ENABLED",
      targetCapacity: 80,
    },
  },
});

const cluster = new aws.ecs.Cluster("cluster", {});

const clusterCapProvider = new aws.ecs.ClusterCapacityProviders(
  "cap-provider",
  {
    clusterName: cluster.name,
    capacityProviders: [capProvider.name],
  }
);

const lb = new awsx.lb.ApplicationLoadBalancer("lb", {
  defaultTargetGroup: {
    targetType: "instance",
  },
});
// const service = new awsx.ecs.FargateService("service", {
//   cluster: cluster.arn,
//   assignPublicIp: true,
//   desiredCount: 2,
//   taskDefinitionArgs: {
//     container: {
//       image: "nginx:latest",
//       cpu: 512,
//       memory: 128,
//       essential: true,
//       portMappings: [
//         {
//           targetGroup: lb.defaultTargetGroup,
//         },
//       ],
//     },
//   },
// });

const td = new aws.ecs.TaskDefinition("td", {
  family: "test-family",
  containerDefinitions: JSON.stringify([
    {
      name: "first",
      image: "nginx:latest",
      cpu: 10,
      memory: 512,
      essential: true,
      portMappings: [
        {
          containerPort: 80,
        },
      ],
    },
  ]),
  volumes: [
    {
      name: "service-storage",
      hostPath: "/ecs/service-storage",
    },
  ],
});

const service = new aws.ecs.Service("service", {
  cluster: cluster.id,
  taskDefinition: td.arn,
  desiredCount: 3,
  orderedPlacementStrategies: [
    {
      type: "binpack",
      field: "cpu",
    },
  ],
  loadBalancers: [
    {
      targetGroupArn: lb.defaultTargetGroup.arn,
      containerName: "first",
      containerPort: 80,
    },
  ],
});

module.exports = {
  databaseAddress: db.address,
  ipn: ipn.publicIp,
  url: lb.loadBalancer.dnsName,
  amiEcs: amiEcs.id,
  // instanceURL: pulumi.interpolate`http://${instance.publicIp}`,
};
