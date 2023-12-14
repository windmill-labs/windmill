const deno = {
  s3_client: `import * as wmill from "npm:windmill-client@1";
import { S3Client } from "https://deno.land/x/s3_lite_client@0.2.0/mod.ts";

type s3object = object;

export async function main(inputFile: s3object) {
  const s3Resource = await wmill.getResource(
    "<PATH_TO_S3_RESOURCE>",
  );
  const s3Client = new S3Client(s3Resource);

  const outputFile = "output/hello.txt"

  // read object from S3
  const getObjectResponse = await s3Client.getObject(inputFile["s3"]);
  const inputObjContent = await getObjectResponse.text();
  console.log(inputObjContent);

  // write object to S3
  await s3Client.putObject(outputFile, "Hello Windmill!");

  // list objects from bucket
  for await (const obj of s3Client.listObjects({ prefix: "output/" })) {
    console.log(obj.key);
  }

  return {
    "s3": outputFile,
  };
}
`
}

export default deno
