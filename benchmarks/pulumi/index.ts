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

// Find the latest Amazon Linux 2 AMI.
const ami = pulumi.output(
  aws.ec2.getAmi({
    owners: ["amazon"],
    mostRecent: true,
    filters: [{ name: "description", values: ["Amazon Linux 2 *"] }],
  })
);

const subnetA = new aws.ec2.Subnet("bench-a", {
  availabilityZone: "us-east-2a",
  vpcId: vpc.id,
  cidrBlock: "10.0.5.0/24",
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

// const instance = new aws.ec2.Instance("windmill-bench-instance", {
//   ami: ami.id,
//   availabilityZone: "us-east-2a",
//   keyName: deployer.keyName,
//   instanceType: "t3.nano",
//   subnetId: subnetA.id,
//   rootBlockDevice: {
//     volumeSize: 20,
//   },
//   vpcSecurityGroupIds: pulumi
//     .all([db.vpcSecurityGroupIds, securityGroup.id])
//     .apply(([db, sg]) => db.concat(sg)),
//   userData: pulumi.interpolate`
//         #!/bin/bash
//         yum update -y
//         yum install -y docker
//         service docker start
//         systemctl enable docker
//         systemctl start docker
//         sudo docker run -p 80:8000 -d -e DATABASE_URL=postgres://postgres:postgres@${db.address}/windmill?sslmode=disable ghcr.io/windmill-labs/windmill:main

//     `,
// });
// const instance = db.address.apply((address) => {
//   // Create and launch an Amazon Linux EC2 instance into the public subnet.

//   return instance;
// });

// const sampleKey = new tailscale.TailnetKey("bench", {
//   ephemeral: false,
//   expiry: 3600,
//   preauthorized: true,
//   reusable: true,
// });

// Export the instance's publicly accessible URL.
module.exports = {
  databaseAddress: db.address,
  databaseUrl: pulumi.interpolate`postgres://${db.username}:${db.password}@${db.address}:5432/${db.dbName}?sslmode=disable`,
  // instanceURL: pulumi.interpolate`http://${instance.publicIp}`,
};
