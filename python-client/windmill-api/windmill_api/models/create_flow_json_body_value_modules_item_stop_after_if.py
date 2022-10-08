from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateFlowJsonBodyValueModulesItemStopAfterIf")


@attr.s(auto_attribs=True)
class CreateFlowJsonBodyValueModulesItemStopAfterIf:
    """
    Attributes:
        expr (str):
        skip_if_stopped (Union[Unset, bool]):
    """

    expr: str
    skip_if_stopped: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        expr = self.expr
        skip_if_stopped = self.skip_if_stopped

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "expr": expr,
            }
        )
        if skip_if_stopped is not UNSET:
            field_dict["skip_if_stopped"] = skip_if_stopped

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        expr = d.pop("expr")

        skip_if_stopped = d.pop("skip_if_stopped", UNSET)

        create_flow_json_body_value_modules_item_stop_after_if = cls(
            expr=expr,
            skip_if_stopped=skip_if_stopped,
        )

        create_flow_json_body_value_modules_item_stop_after_if.additional_properties = d
        return create_flow_json_body_value_modules_item_stop_after_if

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
