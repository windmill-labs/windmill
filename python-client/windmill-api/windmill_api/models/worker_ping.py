from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
import datetime
from dateutil.parser import isoparse
from ..types import UNSET, Unset






T = TypeVar("T", bound="WorkerPing")


@_attrs_define
class WorkerPing:
    """ 
        Attributes:
            worker (str):
            worker_instance (str):
            started_at (datetime.datetime):
            ip (str):
            jobs_executed (int):
            worker_group (str):
            wm_version (str):
            last_ping (Union[Unset, float]):
            custom_tags (Union[Unset, List[str]]):
     """

    worker: str
    worker_instance: str
    started_at: datetime.datetime
    ip: str
    jobs_executed: int
    worker_group: str
    wm_version: str
    last_ping: Union[Unset, float] = UNSET
    custom_tags: Union[Unset, List[str]] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        worker = self.worker
        worker_instance = self.worker_instance
        started_at = self.started_at.isoformat()

        ip = self.ip
        jobs_executed = self.jobs_executed
        worker_group = self.worker_group
        wm_version = self.wm_version
        last_ping = self.last_ping
        custom_tags: Union[Unset, List[str]] = UNSET
        if not isinstance(self.custom_tags, Unset):
            custom_tags = self.custom_tags





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "worker": worker,
            "worker_instance": worker_instance,
            "started_at": started_at,
            "ip": ip,
            "jobs_executed": jobs_executed,
            "worker_group": worker_group,
            "wm_version": wm_version,
        })
        if last_ping is not UNSET:
            field_dict["last_ping"] = last_ping
        if custom_tags is not UNSET:
            field_dict["custom_tags"] = custom_tags

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        worker = d.pop("worker")

        worker_instance = d.pop("worker_instance")

        started_at = isoparse(d.pop("started_at"))




        ip = d.pop("ip")

        jobs_executed = d.pop("jobs_executed")

        worker_group = d.pop("worker_group")

        wm_version = d.pop("wm_version")

        last_ping = d.pop("last_ping", UNSET)

        custom_tags = cast(List[str], d.pop("custom_tags", UNSET))


        worker_ping = cls(
            worker=worker,
            worker_instance=worker_instance,
            started_at=started_at,
            ip=ip,
            jobs_executed=jobs_executed,
            worker_group=worker_group,
            wm_version=wm_version,
            last_ping=last_ping,
            custom_tags=custom_tags,
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
