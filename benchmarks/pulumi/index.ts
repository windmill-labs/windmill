import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";

const vpc = new awsx.ec2.Vpc("bench");

// Create a security group allowing inbound access over port 80 and outbound
// access to anywhere.
const securityGroup = new aws.ec2.SecurityGroup("security-group", {
  vpcId: vpc.vpcId,
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

const deployer = new aws.ec2.KeyPair("deployer", {
  publicKey:
    "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQDRi2xytvAIgfs4f5B8EVrWH3bsu8CI+98iBazq08TuPEkLFP/ZqIVm8ZoTnpPadj2o0DXWHNjiyDAnGwp6W1rcSou4KCxoBViRX0doXBMat5GWs6Za2c+bmKllWUH2jhy4aPrdoI3il2URi7dIMH6WpWZDtD98CFK22rTgiBxhDydFpn3X1ULOI18f/SZEKzcb+iffhP5K/Sjqo65xPiKSl2m40vtaQjn2jTE7vmTD9c/dLv/5A3Sgh7CW2PJe9HR7/8qkzZkRkXtyODOIFfWN+vk0rGvk97IpV+DlSUag+FFeSK5Jn41w9IQV7frPvJAQOZ6eLq2GNv8iO/7QnfhlIUlPeClNJBRgb76FmiuslH+AOBiy/Szy3KUtoV/LO+uzvJxRZVX9DhSvTCm4D2qUTjozS+OSeBVuug70euWe29I7djouAuIbDOFkIkxStjhwISXfgvyWYVh8IMxC6+umZMhMff5CfqzpHY0jy2Tw5r/v/2Iz8EQXNRNAxnqKMPE= rfiszel@rubx1",
});

const rdSubnet = new aws.rds.SubnetGroup("windmill-bench-sng", {
  subnetIds: vpc.publicSubnetIds,
  tags: {
    Name: "bench",
  },
});

const db = new aws.rds.Instance("windmill-bench-rds", {
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

const instance = db.address.apply((address) => {
  // Create and launch an Amazon Linux EC2 instance into the public subnet.
  const instance = new aws.ec2.Instance("windmill-bench-instance", {
    ami: ami.id,
    availabilityZone: "us-east-2a",
    keyName: deployer.keyName,
    instanceType: "t3.nano",
    subnetId: vpc.publicSubnetIds[0],
    rootBlockDevice: {
      volumeSize: 20,
    },
    vpcSecurityGroupIds: db.vpcSecurityGroupIds.apply((ids) => [
      securityGroup.id,
      ...ids,
    ]),
    userData: `
        #!/bin/bash
        yum update -y
        yum install -y docker
        service docker start
        systemctl enable docker
        systemctl start docker
        sudo docker run -p 80:8000 -d -e DATABASE_URL=postgres://postgres:postgres@${address}/windmill?sslmode=disable ghcr.io/windmill-labs/windmill:main
        
    `,
  });
  return instance;
});

// Export the instance's publicly accessible URL.
module.exports = {
  databaseAddress: db.address,
  databaseUrl: pulumi.interpolate`postgres://${db.username}:${db.password}@${db.address}:5432/${db.dbName}?sslmode=disable`,
  instanceURL: pulumi.interpolate`http://${instance.publicIp}`,
};
