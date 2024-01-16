from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import Dict
from ..models.set_default_error_or_recovery_handler_json_body_handler_type import SetDefaultErrorOrRecoveryHandlerJsonBodyHandlerType
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.set_default_error_or_recovery_handler_json_body_extra_args import SetDefaultErrorOrRecoveryHandlerJsonBodyExtraArgs





T = TypeVar("T", bound="SetDefaultErrorOrRecoveryHandlerJsonBody")


@_attrs_define
class SetDefaultErrorOrRecoveryHandlerJsonBody:
    """ 
        Attributes:
            handler_type (SetDefaultErrorOrRecoveryHandlerJsonBodyHandlerType):
            override_existing (bool):
            path (Union[Unset, str]):
            extra_args (Union[Unset, SetDefaultErrorOrRecoveryHandlerJsonBodyExtraArgs]):
            number_of_occurence (Union[Unset, int]):
            number_of_occurence_exact (Union[Unset, bool]):
            workspace_handler_muted (Union[Unset, bool]):
     """

    handler_type: SetDefaultErrorOrRecoveryHandlerJsonBodyHandlerType
    override_existing: bool
    path: Union[Unset, str] = UNSET
    extra_args: Union[Unset, 'SetDefaultErrorOrRecoveryHandlerJsonBodyExtraArgs'] = UNSET
    number_of_occurence: Union[Unset, int] = UNSET
    number_of_occurence_exact: Union[Unset, bool] = UNSET
    workspace_handler_muted: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.set_default_error_or_recovery_handler_json_body_extra_args import SetDefaultErrorOrRecoveryHandlerJsonBodyExtraArgs
        handler_type = self.handler_type.value

        override_existing = self.override_existing
        path = self.path
        extra_args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.extra_args, Unset):
            extra_args = self.extra_args.to_dict()

        number_of_occurence = self.number_of_occurence
        number_of_occurence_exact = self.number_of_occurence_exact
        workspace_handler_muted = self.workspace_handler_muted

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "handler_type": handler_type,
            "override_existing": override_existing,
        })
        if path is not UNSET:
            field_dict["path"] = path
        if extra_args is not UNSET:
            field_dict["extra_args"] = extra_args
        if number_of_occurence is not UNSET:
            field_dict["number_of_occurence"] = number_of_occurence
        if number_of_occurence_exact is not UNSET:
            field_dict["number_of_occurence_exact"] = number_of_occurence_exact
        if workspace_handler_muted is not UNSET:
            field_dict["workspace_handler_muted"] = workspace_handler_muted

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.set_default_error_or_recovery_handler_json_body_extra_args import SetDefaultErrorOrRecoveryHandlerJsonBodyExtraArgs
        d = src_dict.copy()
        handler_type = SetDefaultErrorOrRecoveryHandlerJsonBodyHandlerType(d.pop("handler_type"))




        override_existing = d.pop("override_existing")

        path = d.pop("path", UNSET)

        _extra_args = d.pop("extra_args", UNSET)
        extra_args: Union[Unset, SetDefaultErrorOrRecoveryHandlerJsonBodyExtraArgs]
        if isinstance(_extra_args,  Unset):
            extra_args = UNSET
        else:
            extra_args = SetDefaultErrorOrRecoveryHandlerJsonBodyExtraArgs.from_dict(_extra_args)




        number_of_occurence = d.pop("number_of_occurence", UNSET)

        number_of_occurence_exact = d.pop("number_of_occurence_exact", UNSET)

        workspace_handler_muted = d.pop("workspace_handler_muted", UNSET)

        set_default_error_or_recovery_handler_json_body = cls(
            handler_type=handler_type,
            override_existing=override_existing,
            path=path,
            extra_args=extra_args,
            number_of_occurence=number_of_occurence,
            number_of_occurence_exact=number_of_occurence_exact,
            workspace_handler_muted=workspace_handler_muted,
        )

        set_default_error_or_recovery_handler_json_body.additional_properties = d
        return set_default_error_or_recovery_handler_json_body

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
