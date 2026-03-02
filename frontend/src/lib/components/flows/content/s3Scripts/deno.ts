const deno = {
  s3_client: `import type { S3Object } from "npm:windmill-client@${__pkg__.version}";
import * as wmill from "npm:windmill-client@${__pkg__.version}";
import { S3Client } from "https://deno.land/x/s3_lite_client@0.2.0/mod.ts";

export async function main(inputFile: S3Object) {
  // this will default to the workspace s3 resource
  let args = await wmill.denoS3LightClientSettings();
  // this will use the designated resource
  // let args = await wmill.denoS3LightClientSettings("<PATH_TO_S3_RESOURCE>");
  const s3Client = new S3Client(args);

  const outputFile = "output/hello.txt"

  // read object from S3
  const getObjectResponse = await s3Client.getObject(inputFile.s3);
  const inputObjContent = await getObjectResponse.text();
  console.log(inputObjContent);

  // write object to S3
  await s3Client.putObject(outputFile, "Hello Windmill!");

  // list objects from bucket
  for await (const obj of s3Client.listObjects({ prefix: "output/" })) {
    console.log(obj.key);
  }

  const result: S3Object = {
    s3: outputFile,
  };
  return result;
}
`
}

export default deno
