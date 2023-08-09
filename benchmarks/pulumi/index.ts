import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";
import * as awsx from "@pulumi/awsx";

// Create an AWS resource (S3 Bucket)
const bucket = new aws.s3.Bucket("my-bucket");

// Export the name of the bucket
export const bucketName = bucket.id;

const main = new aws.ec2.Vpc("bench", {
  cidrBlock: "10.0.0.0/16",
  instanceTenancy: "default",
  tags: {
    Name: "bench",
  },
});
