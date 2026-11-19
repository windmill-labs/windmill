"""Windmill.get retries a GET the server rejected under load, with backoff."""

import httpx
import pytest

from wmill.client import Windmill


def _client(statuses, monkeypatch):
    sent, sleeps = [], []
    responses = iter(statuses)

    def handler(request: httpx.Request) -> httpx.Response:
        sent.append(request)
        status, headers = next(responses)
        return httpx.Response(status, headers=headers, json="value")

    monkeypatch.setattr("wmill.client.time.sleep", sleeps.append)
    client = Windmill(base_url="http://wm.test", token="t", workspace="ws")
    client.client = httpx.Client(base_url=client.base_url, transport=httpx.MockTransport(handler))
    return client, sent, sleeps


@pytest.mark.parametrize(
    "statuses, retry_budget, expected",
    [
        ([(503, {}), (429, {"Retry-After": "5"}), (200, {})], None, (200, 3, [1, 5])),
        ([(503, {})] * 4, None, (503, 4, [1, 2, 4])),
        # a Retry-After past the remaining budget ends the retries
        ([(502, {}), (504, {"Retry-After": "120"})], None, (504, 2, [1])),
        ([(429, {"Retry-After": "10"}), (429, {"Retry-After": "10"})], None, (429, 2, [10])),
        ([(503, {"Retry-After": "Fri, 01 Jan 2100 00:00:00 GMT"})], None, (503, 1, [])),
        ([(503, {"Retry-After": "Wed, 21 Oct 2015 07:28:00 GMT"}), (200, {})], None, (200, 2, [0])),
        # "²" is a digit float() rejects: it falls back to the backoff delay
        ([(503, {"Retry-After": "²".encode()}), (200, {})], None, (200, 2, [1])),
        ([(503, {})] * 3, 3, (503, 3, [1, 2])),
        ([(404, {})], None, (404, 1, [])),
        ([(200, {})], None, (200, 1, [])),
    ],
)
def test_get_retries_transient_errors(statuses, retry_budget, expected, monkeypatch):
    client, sent, sleeps = _client(statuses, monkeypatch)
    resp = client.get(
        "/w/ws/variables/get_value/u/a/b", raise_for_status=False, retry_budget=retry_budget
    )
    assert (resp.status_code, len(sent), sleeps) == expected


def test_slack_approval_request_is_not_repeated(monkeypatch):
    # the endpoint posts the approval message: a proxy 502 after it did must not post a second
    client, sent, sleeps = _client([(502, {}), (200, {})], monkeypatch)
    monkeypatch.setenv("WM_JOB_ID", "job")
    monkeypatch.setenv("WM_FLOW_JOB_ID", "flow")
    with pytest.raises(Exception):
        client.request_interactive_slack_approval("u/a/slack", "C1")
    assert (len(sent), sleeps) == (1, [])
