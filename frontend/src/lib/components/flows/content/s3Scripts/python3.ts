const python3 = {
	s3_client: `#requirements:
#boto3==1.34.4
#wmill>=1.229.0

import wmill
from wmill import S3Object
import boto3


def main(input_file: S3Object):
    bucket = wmill.get_resource("<PATH_TO_S3_RESOURCE>")["bucket"]

    # this will default to the workspace s3 resource
    args = wmill.boto3_connection_settings()
    # this will use the designated resource
    # args = wmill.boto3_connection_settings("<PATH_TO_S3_RESOURCE>")
    s3client = boto3.client("s3", **args)

    output_file = "output/hello.txt"

    # read object from S3 and print its content
    input_obj = s3client.get_object(Bucket=bucket, Key=input_file["s3"])["Body"].read()
    print(input_obj)

    # write object to s3
    s3client.put_object(Bucket=bucket, Key=output_file, Body="Hello Windmill!")

    # download file to the job temporary folder:
    s3client.download_file(
        Bucket=bucket, Key=input_file["s3"], Filename="./download.txt"
    )
    with open("./download.txt", mode="rb") as downloaded_file:
        print(downloaded_file.read())

    # upload file from temporary folder to S3
    uploaded_file = "output/uploaded.txt"
    with open("./upload.txt", mode="wb") as file_to_upload:
        file_to_upload.write(str.encode("Hello Windmill!"))
    s3client.upload_file(Bucket=bucket, Key=uploaded_file, Filename="./upload.txt")

    # see https://boto3.amazonaws.com/v1/documentation/api/latest/guide/s3-examples.html
    # and https://boto3.amazonaws.com/v1/documentation/api/latest/reference/services/s3.html
    # for more code examples (listing object, deleting files, etc)
    return [
        S3Object(s3=output_file),
        S3Object(s3=uploaded_file),
    ]
`,

    polars: `#requirements:
#polars==0.20.2
#s3fs==2023.12.0
#wmill>=1.229.0

import wmill
from wmill import S3Object
import polars as pl
import s3fs


def main(input_file: S3Object):
    bucket = wmill.get_resource("<PATH_TO_S3_RESOURCE>")["bucket"]

    # this will default to the workspace s3 resource
    storage_options = wmill.polars_connection_settings().storage_options
    # this will use the designated resource
    # storage_options = wmill.polars_connection_settings("<PATH_TO_S3_RESOURCE>").storage_options

    # input is a parquet file, we use read_parquet in lazy mode.
    # Polars can read various file types, see
    # https://pola-rs.github.io/polars/py-polars/html/reference/io.html
    input_uri = "s3://{}/{}".format(bucket, input_file["s3"])
    input_df = pl.read_parquet(input_uri, storage_options=storage_options).lazy()

    # process the Polars dataframe. See Polars docs:
    # for dataframe: https://pola-rs.github.io/polars/py-polars/html/reference/dataframe/index.html
    # for lazy dataframe: https://pola-rs.github.io/polars/py-polars/html/reference/lazyframe/index.html
    output_df = input_df.collect()
    print(output_df)

    # To write back the result to S3, Polars needs an s3fs connection
    s3 = s3fs.S3FileSystem(**wmill.polars_connection_settings().s3fs_args)
    output_file = "output/result.parquet"
    output_uri = "s3://{}/{}".format(bucket, output_file)
    with s3.open(output_uri, mode="wb") as output_s3:
        # persist the output dataframe back to S3 and return it
        output_df.write_parquet(output_s3)

    return S3Object(s3=output_file)
`,

    duckdb: `#requirements:
#wmill>=1.229.0
#duckdb==0.9.1

import wmill
from wmill import S3Object
import duckdb


def main(input_file: S3Object):
    bucket = wmill.get_resource("u/admin/windmill-cloud-demo")["bucket"]

    # create a DuckDB database in memory
    # see https://duckdb.org/docs/api/python/dbapi
    conn = duckdb.connect()

    # this will default to the workspace s3 resource
    args = wmill.duckdb_connection_settings().connection_settings_str
    # this will use the designated resource
    # args = wmill.duckdb_connection_settings("<PATH_TO_S3_RESOURCE>").connection_settings_str

    # connect duck db to the S3 bucket - this will default to the workspace s3 resource
    conn.execute(args)

    input_uri = "s3://{}/{}".format(bucket, input_file["s3"])
    output_file = "output/result.parquet"
    output_uri = "s3://{}/{}".format(bucket, output_file)

    # Run queries directly on the parquet file
    query_result = conn.sql(
        """
        SELECT * FROM read_parquet('{}')
    """.format(
            input_uri
        )
    )
    query_result.show()

    # Write the result of a query to a different parquet file on S3
    conn.execute(
        """
        COPY (
            SELECT COUNT(*) FROM read_parquet('{input_uri}')
        ) TO '{output_uri}' (FORMAT 'parquet');
    """.format(
            input_uri=input_uri, output_uri=output_uri
        )
    )

    conn.close()
    return S3Object(s3=output_file)
`,
}

export default python3
