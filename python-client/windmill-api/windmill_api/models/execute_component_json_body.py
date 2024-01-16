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
  from ..models.execute_component_json_body_raw_code import ExecuteComponentJsonBodyRawCode
  from ..models.execute_component_json_body_force_viewer_static_fields import ExecuteComponentJsonBodyForceViewerStaticFields





T = TypeVar("T", bound="ExecuteComponentJsonBody")


@_attrs_define
class ExecuteComponentJsonBody:
    """ 
        Attributes:
            component (str):
            args (Any):
            path (Union[Unset, str]):
            raw_code (Union[Unset, ExecuteComponentJsonBodyRawCode]):
            force_viewer_static_fields (Union[Unset, ExecuteComponentJsonBodyForceViewerStaticFields]):
     """

    component: str
    args: Any
    path: Union[Unset, str] = UNSET
    raw_code: Union[Unset, 'ExecuteComponentJsonBodyRawCode'] = UNSET
    force_viewer_static_fields: Union[Unset, 'ExecuteComponentJsonBodyForceViewerStaticFields'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.execute_component_json_body_raw_code import ExecuteComponentJsonBodyRawCode
        from ..models.execute_component_json_body_force_viewer_static_fields import ExecuteComponentJsonBodyForceViewerStaticFields
        component = self.component
        args = self.args
        path = self.path
        raw_code: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.raw_code, Unset):
            raw_code = self.raw_code.to_dict()

        force_viewer_static_fields: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.force_viewer_static_fields, Unset):
            force_viewer_static_fields = self.force_viewer_static_fields.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "component": component,
            "args": args,
        })
        if path is not UNSET:
            field_dict["path"] = path
        if raw_code is not UNSET:
            field_dict["raw_code"] = raw_code
        if force_viewer_static_fields is not UNSET:
            field_dict["force_viewer_static_fields"] = force_viewer_static_fields

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.execute_component_json_body_raw_code import ExecuteComponentJsonBodyRawCode
        from ..models.execute_component_json_body_force_viewer_static_fields import ExecuteComponentJsonBodyForceViewerStaticFields
        d = src_dict.copy()
        component = d.pop("component")

        args = d.pop("args")

        path = d.pop("path", UNSET)

        _raw_code = d.pop("raw_code", UNSET)
        raw_code: Union[Unset, ExecuteComponentJsonBodyRawCode]
        if isinstance(_raw_code,  Unset):
            raw_code = UNSET
        else:
            raw_code = ExecuteComponentJsonBodyRawCode.from_dict(_raw_code)




        _force_viewer_static_fields = d.pop("force_viewer_static_fields", UNSET)
        force_viewer_static_fields: Union[Unset, ExecuteComponentJsonBodyForceViewerStaticFields]
        if isinstance(_force_viewer_static_fields,  Unset):
            force_viewer_static_fields = UNSET
        else:
            force_viewer_static_fields = ExecuteComponentJsonBodyForceViewerStaticFields.from_dict(_force_viewer_static_fields)




        execute_component_json_body = cls(
            component=component,
            args=args,
            path=path,
            raw_code=raw_code,
            force_viewer_static_fields=force_viewer_static_fields,
        )

        execute_component_json_body.additional_properties = d
        return execute_component_json_body

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
