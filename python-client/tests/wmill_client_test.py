import unittest
import wmill
import os


class TestStringMethods(unittest.TestCase):
    _token = "<WM_TOKEN>"
    _workspace = "storage"
    _host = "http://localhost:8000"
    _resource_path = "u/admin/docker_minio"

    def setUp(self):
        os.environ["WM_WORKSPACE"] = self._workspace
        os.environ["WM_TOKEN"] = self._token
        os.environ["BASE_INTERNAL_URL"] = self._host

    def test_duckdb_connection_settings(self):
        settings = wmill.duckdb_connection_settings(self._resource_path)
        self.assertIsNotNone(settings)

        expected_settings_str = """SET home_directory='./';
INSTALL 'httpfs';
SET s3_url_style='path';
SET s3_region='fr-paris';
SET s3_endpoint='localhost:9000';
SET s3_use_ssl=0;
SET s3_access_key_id='IeuKPSYLKTO2h9CWfCVR';
SET s3_secret_access_key='80yMndIMcyXwEujxVNINQbf0tBlIzRaLPyM2m1n4';
"""

        self.assertEqual(settings["connection_settings_str"], expected_settings_str)
        self.assertEqual(settings.connection_settings_str, expected_settings_str)

        settings = wmill.polars_connection_settings(self._resource_path)
        print(settings)

    def test_polars_connection_settings(self):
        settings = wmill.polars_connection_settings(self._resource_path)
        s3fs_args_expected = {
            "endpoint_url": "http://localhost:9000",
            "key": "IeuKPSYLKTO2h9CWfCVR",
            "secret": "80yMndIMcyXwEujxVNINQbf0tBlIzRaLPyM2m1n4",
            "use_ssl": False,
            "cache_regions": False,
            "client_kwargs": {"region_name": "fr-paris"},
        }
        polars_cloud_options_expected = {
            "aws_endpoint_url": "http://localhost:9000",
            "aws_access_key_id": "IeuKPSYLKTO2h9CWfCVR",
            "aws_secret_access_key": "80yMndIMcyXwEujxVNINQbf0tBlIzRaLPyM2m1n4",
            "aws_region": "fr-paris",
            "aws_allow_http": True,
        }
        self.assertEqual(settings["s3fs_args"], s3fs_args_expected)
        self.assertEqual(settings.s3fs_args, s3fs_args_expected)
        self.assertEqual(
            settings["polars_cloud_options"], polars_cloud_options_expected
        )
        self.assertEqual(settings.polars_cloud_options, polars_cloud_options_expected)

    def test_boto3_connection_settings(self):
        settings = wmill.boto3_connection_settings(self._resource_path)
        expected_settings = {
            "endpoint_url": "http://localhost:9000",
            "region_name": "fr-paris",
            "use_ssl": False,
            "aws_access_key_id": "IeuKPSYLKTO2h9CWfCVR",
            "aws_secret_access_key": "80yMndIMcyXwEujxVNINQbf0tBlIzRaLPyM2m1n4",
        }
        self.assertEqual(settings, expected_settings)
        self.assertEqual(settings["endpoint_url"], "http://localhost:9000")
        self.assertEqual(settings.endpoint_url, "http://localhost:9000")


if __name__ == "__main__":
    unittest.main()
