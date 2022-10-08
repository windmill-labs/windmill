from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="GetJobUpdatesResponse200")


@attr.s(auto_attribs=True)
class GetJobUpdatesResponse200:
    """
    Attributes:
        running (Union[Unset, bool]):
        completed (Union[Unset, bool]):
        new_logs (Union[Unset, str]):
    """

    running: Union[Unset, bool] = UNSET
    completed: Union[Unset, bool] = UNSET
    new_logs: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        running = self.running
        completed = self.completed
        new_logs = self.new_logs

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if running is not UNSET:
            field_dict["running"] = running
        if completed is not UNSET:
            field_dict["completed"] = completed
        if new_logs is not UNSET:
            field_dict["new_logs"] = new_logs

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        running = d.pop("running", UNSET)

        completed = d.pop("completed", UNSET)

        new_logs = d.pop("new_logs", UNSET)

        get_job_updates_response_200 = cls(
            running=running,
            completed=completed,
            new_logs=new_logs,
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
