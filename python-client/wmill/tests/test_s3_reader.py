"""Unit tests for S3BufferedReader.read — no network/env needed."""

from wmill.s3_reader import S3BufferedReader

CHUNKS = [b"AAAAAAAAAA", b"BBBBBBBBBB", b"CCCCCCCCCC"]


class _FakeStream:
    """Stands in for the httpx stream response S3BufferedReader reads from."""

    status_code = 200

    def __init__(self, chunks):
        self._chunks = chunks

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass

    def iter_bytes(self):
        return iter(self._chunks)


class _FakeClient:
    """Stands in for the httpx client, so construction never touches the network."""

    def __init__(self, chunks):
        self._chunks = chunks

    def stream(self, method, url, params=None, timeout=None):
        return _FakeStream(self._chunks)


def make_reader(chunks):
    reader = S3BufferedReader("ws", _FakeClient(chunks), "file.txt", None, None)
    reader.__enter__()
    return reader


def test_read_size_returns_exact_bytes():
    reader = make_reader(CHUNKS)
    assert reader.read(5) == b"AAAAA"


def test_read_size_preserves_leftover_across_calls():
    reader = make_reader(CHUNKS)
    assert reader.read(5) == b"AAAAA"
    assert reader.read(5) == b"AAAAA"
    assert reader.read(10) == b"BBBBBBBBBB"
    assert reader.read(7) == b"CCCCCCC"
    assert reader.read(5) == b"CCC"
    assert reader.read(5) == b""


def test_read_zero_returns_empty_without_consuming():
    reader = make_reader(CHUNKS)
    assert reader.read(0) == b""
    assert reader.read(5) == b"AAAAA"


def test_read_all_returns_everything():
    reader = make_reader(CHUNKS)
    assert reader.read(-1) == b"AAAAAAAAAABBBBBBBBBBCCCCCCCCCC"


def test_read_all_after_partial_read():
    reader = make_reader(CHUNKS)
    assert reader.read(5) == b"AAAAA"
    assert reader.read(-1) == b"AAAAABBBBBBBBBBCCCCCCCCCC"


def test_bytes_generator_never_exceeds_50kb_chunk():
    reader = make_reader([b"x" * 65536] * 5)
    total = 0
    while True:
        byte = reader.read(50 * 1024)
        if not byte:
            break
        assert len(byte) <= 50 * 1024
        total += len(byte)
    assert total == 5 * 65536
