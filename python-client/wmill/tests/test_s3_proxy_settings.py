"""The default S3 connection settings go through the S3 proxy only when the server can verify
the token's SigV4 signature and serves the bucket-addressed routes; otherwise they keep the
storage's own credentials."""

import pytest

from wmill.client import Windmill

JOB_TOKEN = "jwt_header.payload.signature"


class _Response:
    def __init__(self, status_code, body):
        self.status_code = status_code
        self._body = body

    def json(self):
        return self._body


def _client(token, status_code=200, body=None):
    client = Windmill(base_url="https://wm.example", token=token, workspace="ws")
    calls = []

    def get(endpoint, raise_for_status=True, **kwargs):
        calls.append(endpoint)
        return _Response(status_code, body if body is not None else {"bucket": "data"})

    def post(endpoint, raise_for_status=True, **kwargs):
        calls.append(endpoint)
        resource = {"endPoint": "s3.example", "useSSL": True, "region": "eu-west-1"}
        return _Response(200, {**resource, "accessKey": "raw", "secretKey": "raw-secret"})

    client.get = get
    client.post = post
    return client, calls


def test_job_token_goes_through_the_proxy():
    client, _ = _client(JOB_TOKEN)
    settings = client.get_boto3_connection_settings()
    assert settings["endpoint_url"] == "https://wm.example/api/w/ws/s3_proxy/_wm_bucket_/_default_"
    assert settings["aws_access_key_id"] == "jwt_header.payload"
    assert settings["aws_secret_access_key"] == "signature"
    assert client.get_polars_connection_settings()["s3_bucket"] == "data"


@pytest.mark.parametrize(
    "token,status_code,body",
    [
        ("personaltokenwithoutdots", 200, None),
        ("jwt_ext_header.payload.signature", 200, None),
        (JOB_TOKEN, 404, None),
        (JOB_TOKEN, 200, {"bucket": None}),
    ],
    ids=["personal token", "external jwt", "older server", "non-S3 storage"],
)
def test_falls_back_to_the_storage_credentials(token, status_code, body):
    client, calls = _client(token, status_code, body)
    settings = client.get_boto3_connection_settings()
    assert settings["aws_access_key_id"] == "raw"
    assert calls[-1].endswith("/job_helpers/v2/s3_resource_info")


def test_explicit_resource_path_never_asks_for_the_proxy():
    client, calls = _client(JOB_TOKEN)
    client.get_boto3_connection_settings("u/admin/s3")
    assert calls == ["/w/ws/job_helpers/v2/s3_resource_info"]
