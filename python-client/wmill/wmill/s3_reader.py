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
        self._buffer = bytearray()

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
        # io.BufferedReader.peek: hand back buffered bytes without consuming
        # them, doing at most one read on the underlying stream — so the amount
        # returned may be more or less than `size`.
        if not self._buffer:
            self._fill(1)
        return bytes(self._buffer)

    def _fill(self, limit):
        # iter_bytes() yields whole HTTP chunks (~64KB), so serving `limit`
        # bytes means accumulating until the buffer holds enough and keeping
        # the remainder of the final chunk for the next call.
        while limit < 0 or len(self._buffer) < limit:
            try:
                self._buffer.extend(next(self._iterator))
            except StopIteration:
                break

    def read(self, size=-1):
        self._fill(size)
        if size < 0:
            result = bytes(self._buffer)
            self._buffer.clear()
            return result
        result = bytes(self._buffer[:size])
        del self._buffer[:size]
        return result

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
