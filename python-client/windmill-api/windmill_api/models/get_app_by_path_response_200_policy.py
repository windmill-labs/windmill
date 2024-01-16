from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..models.get_app_by_path_response_200_policy_execution_mode import GetAppByPathResponse200PolicyExecutionMode
from typing import Dict
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.get_app_by_path_response_200_policy_triggerables import GetAppByPathResponse200PolicyTriggerables





T = TypeVar("T", bound="GetAppByPathResponse200Policy")


@_attrs_define
class GetAppByPathResponse200Policy:
    """ 
        Attributes:
            triggerables (Union[Unset, GetAppByPathResponse200PolicyTriggerables]):
            execution_mode (Union[Unset, GetAppByPathResponse200PolicyExecutionMode]):
            on_behalf_of (Union[Unset, str]):
            on_behalf_of_email (Union[Unset, str]):
     """

    triggerables: Union[Unset, 'GetAppByPathResponse200PolicyTriggerables'] = UNSET
    execution_mode: Union[Unset, GetAppByPathResponse200PolicyExecutionMode] = UNSET
    on_behalf_of: Union[Unset, str] = UNSET
    on_behalf_of_email: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_app_by_path_response_200_policy_triggerables import GetAppByPathResponse200PolicyTriggerables
        triggerables: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.triggerables, Unset):
            triggerables = self.triggerables.to_dict()

        execution_mode: Union[Unset, str] = UNSET
        if not isinstance(self.execution_mode, Unset):
            execution_mode = self.execution_mode.value

        on_behalf_of = self.on_behalf_of
        on_behalf_of_email = self.on_behalf_of_email

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if triggerables is not UNSET:
            field_dict["triggerables"] = triggerables
        if execution_mode is not UNSET:
            field_dict["execution_mode"] = execution_mode
        if on_behalf_of is not UNSET:
            field_dict["on_behalf_of"] = on_behalf_of
        if on_behalf_of_email is not UNSET:
            field_dict["on_behalf_of_email"] = on_behalf_of_email

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_app_by_path_response_200_policy_triggerables import GetAppByPathResponse200PolicyTriggerables
        d = src_dict.copy()
        _triggerables = d.pop("triggerables", UNSET)
        triggerables: Union[Unset, GetAppByPathResponse200PolicyTriggerables]
        if isinstance(_triggerables,  Unset):
            triggerables = UNSET
        else:
            triggerables = GetAppByPathResponse200PolicyTriggerables.from_dict(_triggerables)




        _execution_mode = d.pop("execution_mode", UNSET)
        execution_mode: Union[Unset, GetAppByPathResponse200PolicyExecutionMode]
        if isinstance(_execution_mode,  Unset):
            execution_mode = UNSET
        else:
            execution_mode = GetAppByPathResponse200PolicyExecutionMode(_execution_mode)




        on_behalf_of = d.pop("on_behalf_of", UNSET)

        on_behalf_of_email = d.pop("on_behalf_of_email", UNSET)

        get_app_by_path_response_200_policy = cls(
            triggerables=triggerables,
            execution_mode=execution_mode,
            on_behalf_of=on_behalf_of,
            on_behalf_of_email=on_behalf_of_email,
        )

        get_app_by_path_response_200_policy.additional_properties = d
        return get_app_by_path_response_200_policy

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
