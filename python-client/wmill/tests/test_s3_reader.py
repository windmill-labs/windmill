"""Unit tests for S3BufferedReader: no network or env needed."""

from wmill.s3_reader import S3BufferedReader, bytes_generator

CHUNKS = [b"AAAAAAAAAA", b"BBBBBBBBBB", b"CCCCCCCCCC"]


class _FakeStream:
    """Stands in for the httpx streaming response the reader consumes."""

    status_code = 200

    def __init__(self, chunks):
        self._chunks = chunks

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass

    def iter_bytes(self):
        return iter(self._chunks)


class CountingIterator:
    """Chunk source that records how many times the reader pulled from it."""

    def __init__(self, chunks):
        self._chunks = chunks
        self.pulls = 0

    def __iter__(self):
        for chunk in self._chunks:
            self.pulls += 1
            yield chunk


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


def test_read_size_slices_chunks_and_keeps_the_remainder():
    reader = make_reader(CHUNKS)
    # read(0) must not pull from the stream: the "drain everything" sentinel is
    # a negative size, and widening it to any falsy size would reintroduce the
    # whole-file buffering this reader is built to avoid.
    assert reader.read(0) == b""
    assert reader.read(5) == b"AAAAA"
    assert reader.read(5) == b"AAAAA"
    assert reader.read(10) == b"BBBBBBBBBB"
    assert reader.read(7) == b"CCCCCCC"
    assert reader.read(5) == b"CCC"
    assert reader.read(5) == b""


def test_read_all_drains_both_the_buffer_and_the_stream():
    reader = make_reader(CHUNKS)
    assert reader.read(5) == b"AAAAA"
    assert reader.read(-1) == b"AAAAABBBBBBBBBBCCCCCCCCCC"


def test_bytes_generator_yields_50kb_slices_of_64kb_chunks():
    reader = make_reader([b"x" * 65536] * 5)
    sizes = [len(chunk) for chunk in bytes_generator(reader)]
    assert max(sizes) <= 50 * 1024
    assert sum(sizes) == 5 * 65536


def test_peek_does_not_consume():
    reader = make_reader(CHUNKS)
    assert reader.peek() == b"AAAAAAAAAA"
    assert reader.read(10) == b"AAAAAAAAAA"


def test_read1_stops_after_one_chunk():
    counting = CountingIterator(CHUNKS)
    reader = make_reader(counting)
    # A zero-length read must not touch the stream at all.
    assert reader.read1(0) == b""
    assert counting.pulls == 0
    # read1(-1) must not drain the stream the way read(-1) does.
    assert reader.read1(-1) == b"AAAAAAAAAA"
    assert reader.read1(4) == b"BBBB"
    assert counting.pulls == 2
