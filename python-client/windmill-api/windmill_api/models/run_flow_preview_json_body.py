from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.run_flow_preview_json_body_args import RunFlowPreviewJsonBodyArgs
from ..models.run_flow_preview_json_body_value import RunFlowPreviewJsonBodyValue
from ..types import UNSET, Unset

T = TypeVar("T", bound="RunFlowPreviewJsonBody")


@attr.s(auto_attribs=True)
class RunFlowPreviewJsonBody:
    """
    Attributes:
        value (RunFlowPreviewJsonBodyValue):
        args (RunFlowPreviewJsonBodyArgs):
        path (Union[Unset, str]):
    """

    value: RunFlowPreviewJsonBodyValue
    args: RunFlowPreviewJsonBodyArgs
    path: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        value = self.value.to_dict()

        args = self.args.to_dict()

        path = self.path

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "value": value,
                "args": args,
            }
        )
        if path is not UNSET:
            field_dict["path"] = path

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        value = RunFlowPreviewJsonBodyValue.from_dict(d.pop("value"))

        args = RunFlowPreviewJsonBodyArgs.from_dict(d.pop("args"))

        path = d.pop("path", UNSET)

        run_flow_preview_json_body = cls(
            value=value,
            args=args,
            path=path,
        )

        run_flow_preview_json_body.additional_properties = d
        return run_flow_preview_json_body

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
