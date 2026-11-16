"""The query string get_resume_urls sends, as httpx serialises it."""

import httpx
import pytest

from wmill.client import Windmill


def _sent_query(**kwargs) -> dict:
    sent = []

    def handler(request: httpx.Request) -> httpx.Response:
        sent.append(dict(request.url.params))
        return httpx.Response(200, json={})

    client = Windmill(base_url="http://wm.test", token="t", workspace="ws")
    client.client = httpx.Client(
        base_url=client.base_url, transport=httpx.MockTransport(handler)
    )
    client.get_resume_urls(**kwargs)
    return sent[0]


@pytest.mark.parametrize(
    "kwargs, expected",
    [
        ({}, {}),
        ({"flow_level": True}, {"flow_level": "true"}),
        ({"approver": "alice"}, {"approver": "alice"}),
    ],
)
def test_get_resume_urls_query(kwargs, expected):
    assert _sent_query(**kwargs) == expected
