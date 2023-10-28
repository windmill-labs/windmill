import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";
import * as tailscale from "@pulumi/tailscale";

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
      toPort: 50000,
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
    filters: [
      { name: "description", values: ["Amazon Linux 2 *"] },
      { name: "architecture", values: ["x86_64"] },
    ],
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

const subnetB = new aws.ec2.Subnet("bench-b", {
  availabilityZone: "us-east-2b",
  vpcId: vpc.id,
  cidrBlock: "10.0.6.0/24",
  tags: {
    Name: "bench",
  },
});

const subnetC = new aws.ec2.Subnet("bench-c", {
  availabilityZone: "us-east-2c",
  vpcId: vpc.id,
  cidrBlock: "10.0.7.0/24",
  tags: {
    Name: "bench",
  },
});

new aws.ec2.RouteTableAssociation("bench-a", {
  subnetId: subnetA.id,
  routeTableId: routes.id,
});

new aws.ec2.RouteTableAssociation("bench-b", {
  subnetId: subnetB.id,
  routeTableId: routes.id,
});

new aws.ec2.RouteTableAssociation("bench-c", {
  subnetId: subnetC.id,
  routeTableId: routes.id,
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
  instanceClass: "db.t4g.2xlarge",
  password: "postgres",
  skipFinalSnapshot: true,
  username: "postgres",
  availabilityZone: "us-east-2a",
  performanceInsightsEnabled: true,
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

const ipn = new aws.ec2.Instance("bench-ipn-2", {
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

sudo yum install -y wget unzip
`,
});

// Find the latest Amazon Linux 2 AMI.
const amiEcs = pulumi.output(
  aws.ec2.getAmi({
    owners: ["amazon"],
    mostRecent: true,
    filters: [{ name: "name", values: ["amzn2-ami-ecs-*-arm64-*"] }],
  })
);

const clusterName = "windmill";
const cluster = new aws.ecs.Cluster("cluster", { name: clusterName });

const lt = new aws.ec2.LaunchTemplate("lt2", {
  namePrefix: "windmill",
  imageId: amiEcs.id,
  keyName: deployer.keyName,
  instanceType: "t4g.medium",
  iamInstanceProfile: {
    name: "ecsInstanceRole",
  },

  updateDefaultVersion: true,
  vpcSecurityGroupIds: [securityGroup.id],
  userData: btoa(`#!/bin/bash
echo ECS_CLUSTER=${clusterName} >> /etc/ecs/ecs.config
`),
});

const asg = new aws.autoscaling.Group("asg3", {
  launchTemplate: {
    id: lt.id,
    version: "$Latest",
  },
  minSize: 0,
  maxSize: 2,
  vpcZoneIdentifiers: [subnetA.id],
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

const clusterCapProvider = new aws.ecs.ClusterCapacityProviders(
  "cap-provider",
  {
    clusterName: cluster.name,
    capacityProviders: [capProvider.name],
  }
);

const lb = new awsx.lb.ApplicationLoadBalancer("lb2", {
  defaultTargetGroup: {
    targetType: "instance",
  },
  subnetIds: [subnetA.id, subnetB.id, subnetC.id],
});
// const service = new awsx.ecs.FargateService("service", {
//   cluster: cluster.arn,
//   assignPublicIp: true,
//   desiredCount: 2,
//   taskDefinitionArgs: {
//     container: {
//       image: "nginx:main@sha256:31ce1955e23ec18963e5d3d7357dc68b1a62f54145d19ceff4054d257066129e",
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

db.address.apply((address) => {
  const worker_td = new aws.ecs.TaskDefinition("worker-2", {
    family: "windmill-worker",
    containerDefinitions: JSON.stringify([
      {
        name: "windmill-worker",
        image: "ghcr.io/windmill-labs/windmill-ee:main@sha256:31ce1955e23ec18963e5d3d7357dc68b1a62f54145d19ceff4054d257066129e",
        cpu: 1024,
        memory: 1800,
        essential: true,
        mountPaths: [
          {
            containerPath: "/tmp/windmill/cache",
            sourceVolume: "dependency_cache",
          },
        ],
        environment: [
          { name: "MODE", value: "worker" },
          { name: "NUM_WORKERS", value: "5" },
          { name: "DATABASE_CONNECTIONS", value: "10"},
          { name: "SLEEP_QUEUE", value: "300"},
          // { name: "METRICS_ADDR", value: "true" },
          { name: "RUST_LOG", value: "info" },
          {
            name: "DATABASE_URL",
            value: `postgres://postgres:postgres@${address}/windmill?sslmode=disable`,
          },
        ],

        logConfiguration: {
          logDriver: "awslogs",
          options: {
            "awslogs-group": "windmill-worker",
            "awslogs-region": "us-east-2",
            "awslogs-create-group": "true",
            "awslogs-stream-prefix": "windmill-worker",
          },
        },
        dockerLabels: {
          PROMETHEUS_EXPORTER_PORT: "8001",
        },
        portMappings: [
          {
            containerPort: 8001,
          },
        ],
      },
    ]),
    volumes: [
      {
        name: "dependency_cache",
        dockerVolumeConfiguration: {
          scope: "shared",
          autoprovision: true,
        },
      },
    ],
  });

  const worker_td2 = new aws.ecs.TaskDefinition("worker-3", {
    family: "windmill-worker-2",
    containerDefinitions: JSON.stringify([
      {
        name: "windmill-worker",
        image: "ghcr.io/windmill-labs/windmill-ee:main@sha256:31ce1955e23ec18963e5d3d7357dc68b1a62f54145d19ceff4054d257066129e",
        cpu: 1024,
        memory: 1800,
        essential: true,
        mountPaths: [
          {
            containerPath: "/tmp/windmill/cache",
            sourceVolume: "dependency_cache",
          },
        ],
        environment: [
          { name: "WORKER_GROUP", value: "dedicated" },
          { name: "NUM_WORKERS", value: "10" },
          { name: "DATABASE_CONNECTIONS", value: "15"},
          { name: "SLEEP_QUEUE", value: "300"},
          { name: "MODE", value: "worker" },
          // { name: "METRICS_ADDR", value: "true" },
          { name: "RUST_LOG", value: "info" },
          {
            name: "DATABASE_URL",
            value: `postgres://postgres:postgres@${address}/windmill?sslmode=disable`,
          },
        ],

        logConfiguration: {
          logDriver: "awslogs",
          options: {
            "awslogs-group": "windmill-worker",
            "awslogs-region": "us-east-2",
            "awslogs-create-group": "true",
            "awslogs-stream-prefix": "windmill-worker",
          },
        },
        dockerLabels: {
          PROMETHEUS_EXPORTER_PORT: "8001",
        },
        portMappings: [
          {
            containerPort: 8001,
          },
        ],
      },
    ]),
    volumes: [
      {
        name: "dependency_cache",
        dockerVolumeConfiguration: {
          scope: "shared",
          autoprovision: true,
        },
      },
    ],
  });

  const server_td = new aws.ecs.TaskDefinition("server", {
    family: "windmill-server",
    containerDefinitions: JSON.stringify([
      {
        name: "windmill-server",
        image: "ghcr.io/windmill-labs/windmill-ee:main@sha256:31ce1955e23ec18963e5d3d7357dc68b1a62f54145d19ceff4054d257066129e",
        cpu: 1024,
        memory: 1024,
        essential: true,
        environment: [
          { name: "MODE", value: "server" },
          // { name: "METRICS_ADDR", value: "true" },
          { name: "RUST_LOG", value: "info" },
          { name: "DATABASE_CONNECTIONS", value: "5"},
          {
            name: "DATABASE_URL",
            value: `postgres://postgres:postgres@${address}/windmill?sslmode=disable`,
          },
        ],
        logConfiguration: {
          logDriver: "awslogs",
          options: {
            "awslogs-group": "windmill-server",
            "awslogs-region": "us-east-2",
            "awslogs-create-group": "true",
            "awslogs-stream-prefix": "windmill-server",
          },
        },
        dockerLabels: {
          PROMETHEUS_EXPORTER_PORT: "8001",
        },
        portMappings: [
          {
            containerPort: 8000,
          },
          {
            containerPort: 8001,
          },
        ],
      },
    ]),
  });

  const service_server = new aws.ecs.Service("service-server", {
    cluster: cluster.id,
    taskDefinition: server_td.arn,
    desiredCount: 1,
    forceNewDeployment: true,
    orderedPlacementStrategies: [
      {
        type: "binpack",
        field: "cpu",
      },
    ],
    loadBalancers: [
      {
        targetGroupArn: lb.defaultTargetGroup.arn,
        containerName: "windmill-server",
        containerPort: 8000,
      },
    ],
  });

  const service_worker = new aws.ecs.Service("service-worker", {
    cluster: cluster.id,
    taskDefinition: worker_td.arn,
    desiredCount: 1,
    forceNewDeployment: true,
    orderedPlacementStrategies: [
      {
        type: "binpack",
        field: "cpu",
      },
    ],
  });

  const service_worker2 = new aws.ecs.Service("service-worker-2", {
    cluster: cluster.id,
    taskDefinition: worker_td2.arn,
    desiredCount: 0,
    forceNewDeployment: true,
    orderedPlacementStrategies: [
      {
        type: "binpack",
        field: "cpu",
      },
    ],
  });
});

module.exports = {
  databaseAddress: db.address,
  ipn: ipn.publicIp,
  url: lb.loadBalancer.dnsName,
  amiEcs: amiEcs.id,
  // instanceURL: pulumi.interpolate`http://${instance.publicIp}`,
};
