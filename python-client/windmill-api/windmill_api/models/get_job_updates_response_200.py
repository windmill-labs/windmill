from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="GetJobUpdatesResponse200")


@_attrs_define
class GetJobUpdatesResponse200:
    """ 
        Attributes:
            running (Union[Unset, bool]):
            completed (Union[Unset, bool]):
            new_logs (Union[Unset, str]):
            mem_peak (Union[Unset, int]):
     """

    running: Union[Unset, bool] = UNSET
    completed: Union[Unset, bool] = UNSET
    new_logs: Union[Unset, str] = UNSET
    mem_peak: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        running = self.running
        completed = self.completed
        new_logs = self.new_logs
        mem_peak = self.mem_peak

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if running is not UNSET:
            field_dict["running"] = running
        if completed is not UNSET:
            field_dict["completed"] = completed
        if new_logs is not UNSET:
            field_dict["new_logs"] = new_logs
        if mem_peak is not UNSET:
            field_dict["mem_peak"] = mem_peak

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        running = d.pop("running", UNSET)

        completed = d.pop("completed", UNSET)

        new_logs = d.pop("new_logs", UNSET)

        mem_peak = d.pop("mem_peak", UNSET)

        get_job_updates_response_200 = cls(
            running=running,
            completed=completed,
            new_logs=new_logs,
            mem_peak=mem_peak,
        )

        get_job_updates_response_200.additional_properties = d
        return get_job_updates_response_200

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
