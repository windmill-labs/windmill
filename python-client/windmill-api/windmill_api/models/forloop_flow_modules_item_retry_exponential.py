from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="ForloopFlowModulesItemRetryExponential")


@_attrs_define
class ForloopFlowModulesItemRetryExponential:
    """ 
        Attributes:
            attempts (Union[Unset, int]):
            multiplier (Union[Unset, int]):
            seconds (Union[Unset, int]):
            random_factor (Union[Unset, int]):
     """

    attempts: Union[Unset, int] = UNSET
    multiplier: Union[Unset, int] = UNSET
    seconds: Union[Unset, int] = UNSET
    random_factor: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        attempts = self.attempts
        multiplier = self.multiplier
        seconds = self.seconds
        random_factor = self.random_factor

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if attempts is not UNSET:
            field_dict["attempts"] = attempts
        if multiplier is not UNSET:
            field_dict["multiplier"] = multiplier
        if seconds is not UNSET:
            field_dict["seconds"] = seconds
        if random_factor is not UNSET:
            field_dict["random_factor"] = random_factor

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        attempts = d.pop("attempts", UNSET)

        multiplier = d.pop("multiplier", UNSET)

        seconds = d.pop("seconds", UNSET)

        random_factor = d.pop("random_factor", UNSET)

        forloop_flow_modules_item_retry_exponential = cls(
            attempts=attempts,
            multiplier=multiplier,
            seconds=seconds,
            random_factor=random_factor,
        )

        forloop_flow_modules_item_retry_exponential.additional_properties = d
        return forloop_flow_modules_item_retry_exponential

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
