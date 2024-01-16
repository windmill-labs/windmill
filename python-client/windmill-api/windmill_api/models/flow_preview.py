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
  from ..models.flow_preview_value import FlowPreviewValue
  from ..models.flow_preview_restarted_from import FlowPreviewRestartedFrom
  from ..models.flow_preview_args import FlowPreviewArgs





T = TypeVar("T", bound="FlowPreview")


@_attrs_define
class FlowPreview:
    """ 
        Attributes:
            value (FlowPreviewValue):
            args (FlowPreviewArgs):
            path (Union[Unset, str]):
            tag (Union[Unset, str]):
            restarted_from (Union[Unset, FlowPreviewRestartedFrom]):
     """

    value: 'FlowPreviewValue'
    args: 'FlowPreviewArgs'
    path: Union[Unset, str] = UNSET
    tag: Union[Unset, str] = UNSET
    restarted_from: Union[Unset, 'FlowPreviewRestartedFrom'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.flow_preview_value import FlowPreviewValue
        from ..models.flow_preview_restarted_from import FlowPreviewRestartedFrom
        from ..models.flow_preview_args import FlowPreviewArgs
        value = self.value.to_dict()

        args = self.args.to_dict()

        path = self.path
        tag = self.tag
        restarted_from: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.restarted_from, Unset):
            restarted_from = self.restarted_from.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "value": value,
            "args": args,
        })
        if path is not UNSET:
            field_dict["path"] = path
        if tag is not UNSET:
            field_dict["tag"] = tag
        if restarted_from is not UNSET:
            field_dict["restarted_from"] = restarted_from

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.flow_preview_value import FlowPreviewValue
        from ..models.flow_preview_restarted_from import FlowPreviewRestartedFrom
        from ..models.flow_preview_args import FlowPreviewArgs
        d = src_dict.copy()
        value = FlowPreviewValue.from_dict(d.pop("value"))




        args = FlowPreviewArgs.from_dict(d.pop("args"))




        path = d.pop("path", UNSET)

        tag = d.pop("tag", UNSET)

        _restarted_from = d.pop("restarted_from", UNSET)
        restarted_from: Union[Unset, FlowPreviewRestartedFrom]
        if isinstance(_restarted_from,  Unset):
            restarted_from = UNSET
        else:
            restarted_from = FlowPreviewRestartedFrom.from_dict(_restarted_from)




        flow_preview = cls(
            value=value,
            args=args,
            path=path,
            tag=tag,
            restarted_from=restarted_from,
        )

        flow_preview.additional_properties = d
        return flow_preview

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
