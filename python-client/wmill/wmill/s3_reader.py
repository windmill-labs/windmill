from io import BufferedReader, BytesIO
from json import JSONDecodeError

import httpx


class S3BufferedReader(BufferedReader):
    def __init__(self, workspace: str, windmill_client: httpx.Client, file_key: str, s3_resource_path: str | None, storage: str | None):
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


def bytes_generator(buffered_reader: BufferedReader | BytesIO):
    while True:
        byte = buffered_reader.read(50 * 1024)
        if not byte:
            break
        yield byte
