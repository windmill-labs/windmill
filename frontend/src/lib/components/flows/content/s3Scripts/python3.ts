const python3 = {
	push: `import os
import boto3
from typing import TypedDict, Union

class s3(TypedDict):
    port: int
    bucket: str
    region: str
    useSSL: bool
    endPoint: str
    accessKey: str
    pathStyle: bool
    secretKey: str

def main(
    s3_config: s3,
    object_name: str,
    data: Union[str, bytes],
    base_path: str = "windmill",
):
    # flow_path/schedule_path_or_manual/flow_step_id/ts_job_id
    object_path = os.getenv("WM_OBJECT_PATH")

    full_path = base_path + "/" + object_path + "/" + object_name

    s3Client = boto3.client(
        's3',
        region_name=s3_config['region'],
        aws_access_key_id=s3_config['accessKey'],
        aws_secret_access_key=s3_config['secretKey']
    )

    s3Client.put_object(Body=data, Bucket=s3_config['bucket'], Key=full_path)

    return full_path`,
	pull: `import boto3
from typing import TypedDict

class s3(TypedDict):
    port: int
    bucket: str
    region: str
    useSSL: bool
    endPoint: str
    accessKey: str
    pathStyle: bool
    secretKey: str

def main(
    s3_config: s3,
    object_path: str,
):

    s3Client = boto3.client(
        's3',
        region_name=s3_config['region'],
        aws_access_key_id=s3_config['accessKey'],
        aws_secret_access_key=s3_config['secretKey']
    )

    obj = s3Client.get_object(Bucket=s3_config["bucket"], Key=object_path)["Body"].read()

    # for instance, if it is a text file
    return str(obj, encoding="utf-8")`,
	aggregate: `import os
import boto3

from typing import TypedDict

class s3(TypedDict):
    port: int
    bucket: str
    region: str
    useSSL: bool
    endPoint: str
    accessKey: str
    pathStyle: bool
    secretKey: str


def main(
    s3_config: s3,
    object_path: str,
    last_n = 10,
):
    # object path assumed to be of the form windmill/flow_path/schedule_path_or_manual/flow_step_id/ts_job_id/**
    prefix = "/".join(object_path.split("/")[:4])

    s3Client = boto3.client(
        "s3",
        region_name=s3_config["region"],
        aws_access_key_id=s3_config["accessKey"],
        aws_secret_access_key=s3_config["secretKey"],
    )

    # will return the object keys of the last_n jobs
    objs = {}
    for content in s3Client.list_objects(
        Bucket=s3_config["bucket"], Prefix=prefix
    )["Contents"]:
        obj_key = content["Key"]
        ts = int(obj_key.split("/")[4].split("_")[0])
        if ts in objs:
            objs[ts].append(obj_key)
        else:
            objs[ts] = [obj_key]

    tss = sorted(objs.keys(), reverse=True)[:last_n]

    final_objs = []
    for ts in tss:
        final_objs.extend(objs[ts])

    return final_objs`
}

export default python3
