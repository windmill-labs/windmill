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
  from ..models.edit_error_handler_json_body_error_handler_extra_args import EditErrorHandlerJsonBodyErrorHandlerExtraArgs





T = TypeVar("T", bound="EditErrorHandlerJsonBody")


@_attrs_define
class EditErrorHandlerJsonBody:
    """ 
        Attributes:
            error_handler (Union[Unset, str]):
            error_handler_extra_args (Union[Unset, EditErrorHandlerJsonBodyErrorHandlerExtraArgs]):
            error_handler_muted_on_cancel (Union[Unset, bool]):
     """

    error_handler: Union[Unset, str] = UNSET
    error_handler_extra_args: Union[Unset, 'EditErrorHandlerJsonBodyErrorHandlerExtraArgs'] = UNSET
    error_handler_muted_on_cancel: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.edit_error_handler_json_body_error_handler_extra_args import EditErrorHandlerJsonBodyErrorHandlerExtraArgs
        error_handler = self.error_handler
        error_handler_extra_args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.error_handler_extra_args, Unset):
            error_handler_extra_args = self.error_handler_extra_args.to_dict()

        error_handler_muted_on_cancel = self.error_handler_muted_on_cancel

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if error_handler is not UNSET:
            field_dict["error_handler"] = error_handler
        if error_handler_extra_args is not UNSET:
            field_dict["error_handler_extra_args"] = error_handler_extra_args
        if error_handler_muted_on_cancel is not UNSET:
            field_dict["error_handler_muted_on_cancel"] = error_handler_muted_on_cancel

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.edit_error_handler_json_body_error_handler_extra_args import EditErrorHandlerJsonBodyErrorHandlerExtraArgs
        d = src_dict.copy()
        error_handler = d.pop("error_handler", UNSET)

        _error_handler_extra_args = d.pop("error_handler_extra_args", UNSET)
        error_handler_extra_args: Union[Unset, EditErrorHandlerJsonBodyErrorHandlerExtraArgs]
        if isinstance(_error_handler_extra_args,  Unset):
            error_handler_extra_args = UNSET
        else:
            error_handler_extra_args = EditErrorHandlerJsonBodyErrorHandlerExtraArgs.from_dict(_error_handler_extra_args)




        error_handler_muted_on_cancel = d.pop("error_handler_muted_on_cancel", UNSET)

        edit_error_handler_json_body = cls(
            error_handler=error_handler,
            error_handler_extra_args=error_handler_extra_args,
            error_handler_muted_on_cancel=error_handler_muted_on_cancel,
        )

        edit_error_handler_json_body.additional_properties = d
        return edit_error_handler_json_body

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
