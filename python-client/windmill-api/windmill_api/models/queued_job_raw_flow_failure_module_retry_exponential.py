from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="QueuedJobRawFlowFailureModuleRetryExponential")


@attr.s(auto_attribs=True)
class QueuedJobRawFlowFailureModuleRetryExponential:
    """
    Attributes:
        attempts (Union[Unset, int]):
        multiplier (Union[Unset, int]):
        seconds (Union[Unset, int]):
    """

    attempts: Union[Unset, int] = UNSET
    multiplier: Union[Unset, int] = UNSET
    seconds: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        attempts = self.attempts
        multiplier = self.multiplier
        seconds = self.seconds

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if attempts is not UNSET:
            field_dict["attempts"] = attempts
        if multiplier is not UNSET:
            field_dict["multiplier"] = multiplier
        if seconds is not UNSET:
            field_dict["seconds"] = seconds

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        attempts = d.pop("attempts", UNSET)

        multiplier = d.pop("multiplier", UNSET)

        seconds = d.pop("seconds", UNSET)

        queued_job_raw_flow_failure_module_retry_exponential = cls(
            attempts=attempts,
            multiplier=multiplier,
            seconds=seconds,
        )

        queued_job_raw_flow_failure_module_retry_exponential.additional_properties = d
        return queued_job_raw_flow_failure_module_retry_exponential

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
