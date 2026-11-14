from wmill.client import Windmill


def test_get_resume_urls_omits_empty_approver():
    client = Windmill(base_url="https://wm.example", token="t", workspace="ws")
    calls = []
    client.get = lambda endpoint, **kw: calls.append(kw.get("params")) or type(
        "R", (), {"json": lambda self: {}})()
    client.get_resume_urls(flow_level=True)
    assert calls == [{"flow_level": True}]
