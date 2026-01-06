from io import BufferedReader, BytesIO
from typing import Optional, Union

import httpx


class S3BufferedReader(BufferedReader):
    """Streaming buffered reader for S3 files via Windmill's S3 proxy.

    Args:
        workspace: Windmill workspace ID
        windmill_client: HTTP client for Windmill API
        file_key: S3 file key/path
        s3_resource_path: Optional path to S3 resource configuration
        storage: Optional storage backend identifier
    """
    def __init__(self, workspace: str, windmill_client: httpx.Client, file_key: str, s3_resource_path: Optional[str], storage: Optional[str]):
        params = {
            "file_key": file_key,
        }
        if s3_resource_path is not None:
            params["s3_resource_path"] = s3_resource_path
        if storage is not None:
            params["storage"] = storage
        self._context_manager = windmill_client.stream(
            "GET",
            f"/w/{workspace}/job_helpers/download_s3_file",
            params=params,
            timeout=None,
        )

    def __enter__(self):
        reader = self._context_manager.__enter__()
        if reader.status_code >= 400:
            error_bytes = reader.read()
            try:
                error_text = error_bytes.decode('utf-8')
            except UnicodeDecodeError:
                error_text = str(error_bytes)
            raise httpx.HTTPStatusError(
                f"Failed to load S3 file: {reader.status_code} {reader.reason_phrase} - {error_text}",
                request=reader.request,
                response=reader
            )
        self._iterator = reader.iter_bytes()
        return self

    def peek(self, size=0):
        raise Exception("Not implemented, use read() instead")

    def read(self, size=-1):
        read_result = []
        if size < 0:
            for b in self._iterator:
                read_result.append(b)
        else:
            for i in range(size):
                try:
                    b = self._iterator.__next__()
                except StopIteration:
                    break
                read_result.append(b)

        return b"".join(read_result)

    def read1(self, size=-1):
        return self.read(size)

    def __exit__(self, *args):
        self._context_manager.__exit__(*args)


def bytes_generator(buffered_reader: Union[BufferedReader, BytesIO]):
    """Yield 50KB chunks from a buffered reader.

    Args:
        buffered_reader: File-like object to read from

    Yields:
        Bytes chunks of up to 50KB
    """
    while True:
        byte = buffered_reader.read(50 * 1024)
        if not byte:
            break
        yield byte
