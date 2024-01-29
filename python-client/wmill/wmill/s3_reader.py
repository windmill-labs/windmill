from io import BufferedReader
from json import JSONDecodeError

import httpx


class S3BufferedReader(BufferedReader):
    def __init__(self, workspace: str, windmill_client: httpx.Client, file_key: str, s3_resource_path: str | None):
        self._workspace = workspace
        self._client = windmill_client
        self._file_key = file_key
        self._s3_resource_path = s3_resource_path
        self._file_size: int | None = None

        self._part_number: int | None = 0
        self._current_chunk: list[int] = []
        self._position_in_chunk = 0

    def peek(self, size=0):
        read_result = []

        if size > 0 or (
            len(self._current_chunk) > self._position_in_chunk
            and len(self._current_chunk) > self._position_in_chunk + size
        ):
            payload_to_return = self._current_chunk[self._position_in_chunk : (self._position_in_chunk + size)]
            read_result += payload_to_return
            return bytes(read_result)

        if self._position_in_chunk < len(self._current_chunk):
            payload_to_return = self._current_chunk[self._position_in_chunk :]
            read_result += bytes(payload_to_return)

        previous_chunk = self._current_chunk
        previous_part_number = self._part_number
        previous_position_in_chunk = self._position_in_chunk
        try:
            while len(read_result) < size or self._part_number is not None:
                self._download_new_chunk()
                if size > 0 and size - len(read_result) < len(self._current_chunk):
                    payload_to_return = self._current_chunk[: (size - len(read_result))]
                    self._position_in_chunk = size - len(read_result)
                    read_result += bytes(payload_to_return)
                    break

                read_result += bytes(self._current_chunk)
                if self._part_number is None:
                    break
        finally:
            # always roll back the changes to the stream state
            self._current_chunk = previous_chunk
            self._part_number = previous_part_number
            self._position_in_chunk = previous_position_in_chunk
        return read_result

    def read(self, size=-1):
        read_result = []

        if size > 0 and (
            len(self._current_chunk) > self._position_in_chunk
            and len(self._current_chunk) > self._position_in_chunk + size
        ):
            payload_to_return = self._current_chunk[self._position_in_chunk : (self._position_in_chunk + size)]
            self._position_in_chunk += size
            read_result += payload_to_return
            return bytes(read_result)

        if self._position_in_chunk < len(self._current_chunk):
            payload_to_return = self._current_chunk[self._position_in_chunk :]
            self._position_in_chunk = len(self._current_chunk)
            read_result += payload_to_return

        previous_chunk = self._current_chunk
        previous_part_number = self._part_number
        previous_position_in_chunk = self._position_in_chunk
        try:
            while len(read_result) < size or self._part_number is not None:
                self._download_new_chunk()
                if size > 0 and size - len(read_result) < len(self._current_chunk):
                    payload_to_return = self._current_chunk[: (size - len(read_result))]
                    self._position_in_chunk = size - len(read_result)
                    read_result += payload_to_return
                    break

                read_result += self._current_chunk
                if self._part_number is None:
                    break
        except Exception as e:
            # roll back the changes to the stream state
            self._current_chunk = previous_chunk
            self._part_number = previous_part_number
            self._position_in_chunk = previous_position_in_chunk
            raise e
        return bytes(read_result)

    def read1(self, size=-1):
        read_result = []

        if size < 0:
            payload_to_return = self._current_chunk[self._position_in_chunk :]
            self._position_in_chunk = len(self._current_chunk)
            read_result += payload_to_return
            return bytes(read_result)

        if size > 0 and len(self._current_chunk) > self._position_in_chunk:
            end_byte = min(self._position_in_chunk + size, len(self._current_chunk))
            payload_to_return = self._current_chunk[self._position_in_chunk : end_byte]
            self._position_in_chunk = end_byte
            read_result += payload_to_return
            return bytes(read_result)

        # no bytes in current buffer, load a new chunk
        self._download_new_chunk()
        end_byte = min(size, len(self._current_chunk))
        payload_to_return = self._current_chunk[:end_byte]
        self._position_in_chunk = end_byte
        read_result += payload_to_return
        return bytes(read_result)

    def close(self):
        self._part_number = 0
        self._current_chunk = []
        self._position_in_chunk = 0

    def _download_new_chunk(
        self,
    ):
        try:
            raw_response = self._client.post(
                f"/w/{self._workspace}/job_helpers/multipart_download_s3_file",
                json={
                    "file_key": self._file_key,
                    "part_number": self._part_number,
                    "file_size": self._file_size,
                    "s3_resource_path": self._s3_resource_path,
                },
            )
            try:
                raw_response.raise_for_status()
            except httpx.HTTPStatusError as err:
                raise Exception(f"{err.request.url}: {err.response.status_code}, {err.response.text}")
            response = raw_response.json()
        except JSONDecodeError as e:
            raise Exception("Could not generate download S3 file part") from e

        self._current_chunk = response["part_content"]
        self._part_number = response["next_part_number"]
        self._file_size = response["file_size"]
        self._position_in_chunk = 0
