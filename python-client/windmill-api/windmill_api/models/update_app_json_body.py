from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..types import UNSET, Unset
from typing import Dict

if TYPE_CHECKING:
  from ..models.update_app_json_body_policy import UpdateAppJsonBodyPolicy





T = TypeVar("T", bound="UpdateAppJsonBody")


@_attrs_define
class UpdateAppJsonBody:
    """ 
        Attributes:
            path (Union[Unset, str]):
            summary (Union[Unset, str]):
            value (Union[Unset, Any]):
            policy (Union[Unset, UpdateAppJsonBodyPolicy]):
            deployment_message (Union[Unset, str]):
     """

    path: Union[Unset, str] = UNSET
    summary: Union[Unset, str] = UNSET
    value: Union[Unset, Any] = UNSET
    policy: Union[Unset, 'UpdateAppJsonBodyPolicy'] = UNSET
    deployment_message: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.update_app_json_body_policy import UpdateAppJsonBodyPolicy
        path = self.path
        summary = self.summary
        value = self.value
        policy: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.policy, Unset):
            policy = self.policy.to_dict()

        deployment_message = self.deployment_message

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if path is not UNSET:
            field_dict["path"] = path
        if summary is not UNSET:
            field_dict["summary"] = summary
        if value is not UNSET:
            field_dict["value"] = value
        if policy is not UNSET:
            field_dict["policy"] = policy
        if deployment_message is not UNSET:
            field_dict["deployment_message"] = deployment_message

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.update_app_json_body_policy import UpdateAppJsonBodyPolicy
        d = src_dict.copy()
        path = d.pop("path", UNSET)

        summary = d.pop("summary", UNSET)

        value = d.pop("value", UNSET)

        _policy = d.pop("policy", UNSET)
        policy: Union[Unset, UpdateAppJsonBodyPolicy]
        if isinstance(_policy,  Unset):
            policy = UNSET
        else:
            policy = UpdateAppJsonBodyPolicy.from_dict(_policy)




        deployment_message = d.pop("deployment_message", UNSET)

        update_app_json_body = cls(
            path=path,
            summary=summary,
            value=value,
            policy=policy,
            deployment_message=deployment_message,
        )

        update_app_json_body.additional_properties = d
        return update_app_json_body

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
