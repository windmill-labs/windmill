from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="GetScriptDeploymentStatusResponse200")


@attr.s(auto_attribs=True)
class GetScriptDeploymentStatusResponse200:
    """
    Attributes:
        lock (Union[Unset, str]):
        lock_error_logs (Union[Unset, str]):
    """

    lock: Union[Unset, str] = UNSET
    lock_error_logs: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        lock = self.lock
        lock_error_logs = self.lock_error_logs

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if lock is not UNSET:
            field_dict["lock"] = lock
        if lock_error_logs is not UNSET:
            field_dict["lock_error_logs"] = lock_error_logs

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        lock = d.pop("lock", UNSET)

        lock_error_logs = d.pop("lock_error_logs", UNSET)

        get_script_deployment_status_response_200 = cls(
            lock=lock,
            lock_error_logs=lock_error_logs,
        )

        get_script_deployment_status_response_200.additional_properties = d
        return get_script_deployment_status_response_200

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
