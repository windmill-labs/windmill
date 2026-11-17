"""Windmill.get retries a GET the server rejected under load, with backoff."""

import httpx
import pytest

from wmill.client import Windmill


def _get(statuses, monkeypatch):
    sent, sleeps = [], []
    responses = iter(statuses)

    def handler(request: httpx.Request) -> httpx.Response:
        sent.append(request)
        status, headers = next(responses)
        return httpx.Response(status, headers=headers, json="value")

    monkeypatch.setattr("wmill.client.time.sleep", sleeps.append)
    client = Windmill(base_url="http://wm.test", token="t", workspace="ws")
    client.client = httpx.Client(base_url=client.base_url, transport=httpx.MockTransport(handler))
    resp = client.get("/w/ws/variables/get_value/u/a/b", raise_for_status=False)
    return resp.status_code, len(sent), sleeps


@pytest.mark.parametrize(
    "statuses, expected",
    [
        ([(503, {}), (429, {"Retry-After": "5"}), (200, {})], (200, 3, [1, 5])),
        ([(503, {})] * 4, (503, 4, [1, 2, 4])),
        ([(502, {}), (504, {"Retry-After": "120"}), (200, {})], (200, 3, [1, 60])),
        (
            [
                (503, {"Retry-After": "Fri, 01 Jan 2100 00:00:00 GMT"}),
                (503, {"Retry-After": "Wed, 21 Oct 2015 07:28:00 GMT"}),
                (200, {}),
            ],
            (200, 3, [60, 0]),
        ),
        ([(404, {})], (404, 1, [])),
        ([(200, {})], (200, 1, [])),
    ],
)
def test_get_retries_transient_errors(statuses, expected, monkeypatch):
    assert _get(statuses, monkeypatch) == expected
