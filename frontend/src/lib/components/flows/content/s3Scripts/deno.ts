const deno = {
	push: `import { S3Client } from "https://deno.land/x/s3_lite_client@0.2.0/mod.ts";

type S3 = {
  port: number;
  bucket: string;
  region: string;
  useSSL: boolean;
  endPoint: string;
  accessKey: string;
  pathStyle: boolean;
  secretKey: string;
};

export async function main(
  s3Config: S3,
  basePath = "windmill",
  objectName: string,
  data: string | Uint8Array | ReadableStream<Uint8Array>,
) {
  // flow_path/schedule_path_or_manual/flow_step_id/ts_job_id
  const objectPath = Deno.env.get("WM_OBJECT_PATH"); 

  const fullPath = basePath + "/" + objectPath + "/" + objectName;

  const s3Client = new S3Client(s3Config);

  await s3Client.putObject(fullPath, data);

  return fullPath;
}`,
	pull: `import { S3Client } from "https://deno.land/x/s3_lite_client@0.2.0/mod.ts";

type S3 = {
  port: number;
  bucket: string;
  region: string;
  useSSL: boolean;
  endPoint: string;
  accessKey: string;
  pathStyle: boolean;
  secretKey: string;
};

export async function main(
  s3Config: S3,
  objectPath: string,
) {

  const s3Client = new S3Client(s3Config);

  const response = await s3Client.getObject(objectPath)
  // for instance, if it is a text file
  const result = await response.text()
  return result
}`,
	aggregate: `import { S3Client } from "https://deno.land/x/s3_lite_client@0.2.0/mod.ts";

type S3 = {
  port: number;
  bucket: string;
  region: string;
  useSSL: boolean;
  endPoint: string;
  accessKey: string;
  pathStyle: boolean;
  secretKey: string;
};

export async function main(
  s3Config: S3,
  objectPath: string,
  last_n = 10,
) {

  // object path assumed to be of the form windmill/flow_path/schedule_path_or_manual/flow_step_id/ts_job_id/**
  const prefix = objectPath.split("/").slice(0, 4).join("/")

  const s3Client = new S3Client(s3Config);
  
  // will return the object keys of the last_n jobs
  const objs = {};
  for await (const entry of s3Client.listObjects({ prefix })) {
    const obj_key = entry.key
    const ts = parseInt(obj_key.split("/")[4].split("_")[0])
    if (ts in objs) {
      objs[ts].append()
    } else {
      objs[ts] = [obj_key]
    }
  }

  const tss = Object.keys(objs).sort().slice(-last_n)


  const final_objs = []
  for (const ts of tss) {
    final_objs.push(...objs[ts])
  }

  return final_objs;
}`
}

export default deno
