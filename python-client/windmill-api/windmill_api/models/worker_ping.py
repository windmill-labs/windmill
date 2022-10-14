import datetime
from typing import Any, Dict, List, Type, TypeVar

import attr
from dateutil.parser import isoparse

T = TypeVar("T", bound="WorkerPing")


@attr.s(auto_attribs=True)
class WorkerPing:
    """
    Attributes:
        worker (str):
        worker_instance (str):
        ping_at (datetime.datetime):
        started_at (datetime.datetime):
        ip (str):
        jobs_executed (int):
    """

    worker: str
    worker_instance: str
    ping_at: datetime.datetime
    started_at: datetime.datetime
    ip: str
    jobs_executed: int
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        worker = self.worker
        worker_instance = self.worker_instance
        ping_at = self.ping_at.isoformat()

        started_at = self.started_at.isoformat()

        ip = self.ip
        jobs_executed = self.jobs_executed

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "worker": worker,
                "worker_instance": worker_instance,
                "ping_at": ping_at,
                "started_at": started_at,
                "ip": ip,
                "jobs_executed": jobs_executed,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        worker = d.pop("worker")

        worker_instance = d.pop("worker_instance")

        ping_at = isoparse(d.pop("ping_at"))

        started_at = isoparse(d.pop("started_at"))

        ip = d.pop("ip")

        jobs_executed = d.pop("jobs_executed")

        worker_ping = cls(
            worker=worker,
            worker_instance=worker_instance,
            ping_at=ping_at,
            started_at=started_at,
            ip=ip,
            jobs_executed=jobs_executed,
        )

        worker_ping.additional_properties = d
        return worker_ping

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
