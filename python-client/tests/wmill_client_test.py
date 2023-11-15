import unittest
import wmill
import os


class TestStringMethods(unittest.TestCase):
    _token = "<TOKEN>"
    _workspace = "<WORKSPACE>"

    def setUp(self):
        os.environ["WM_WORKSPACE"] = self._workspace
        os.environ["WM_TOKEN"] = self._token

    def test_duckdb_connection_settings(self):
        s3_resource = {
            "port": 9000,
            "bucket": "windmill",
            "region": "fr-paris",
            "useSSL": False,
            "endPoint": "localhost:9000",
            "accessKey": "ACCESS_KEY",
            "pathStyle": True,
            "secretKey": "SECRET_KEY",
        }

        settings = wmill.duckdb_connection_settings(s3_resource)
        self.assertIsNotNone(settings)

        expected_settings_str = """SET home_directory='./shared/';
INSTALL 'httpfs';
SET s3_url_style='path';
SET s3_region='fr-paris';
SET s3_endpoint='localhost:9000';
SET s3_use_ssl=0;
SET s3_access_key_id='ACCESS_KEY';
SET s3_secret_access_key='SECRET_KEY';
"""
        self.assertEqual(settings, {"connection_settings_str": expected_settings_str})

        settings = wmill.polars_connection_settings(s3_resource)
        print(settings)

    def test_polars_connection_settings(self):
        s3_resource = {
            "port": 9000,
            "bucket": "windmill",
            "region": "fr-paris",
            "useSSL": False,
            "endPoint": "localhost:9000",
            "accessKey": "ACCESS_KEY",
            "pathStyle": True,
            "secretKey": "SECRET_KEY",
        }

        settings = wmill.polars_connection_settings(s3_resource)
        print(settings)
        expected_settings = {
            "cache_regions": False,
            "client_kwargs": {"region_name": "fr-paris"},
            "endpoint_url": "localhost:9000",
            "key": "ACCESS_KEY",
            "secret": "SECRET_KEY",
            "use_ssl": False,
        }
        self.assertEqual(settings, expected_settings)


if __name__ == "__main__":
    unittest.main()
