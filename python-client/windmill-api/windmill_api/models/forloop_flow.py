from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.forloop_flow_type import ForloopFlowType
from ..models.javascript_transform import JavascriptTransform
from ..models.static_transform import StaticTransform

T = TypeVar("T", bound="ForloopFlow")


@attr.s(auto_attribs=True)
class ForloopFlow:
    """
    Attributes:
        iterator (Union[JavascriptTransform, StaticTransform]):
        skip_failures (bool):
        type (ForloopFlowType):
    """

    iterator: Union[JavascriptTransform, StaticTransform]
    skip_failures: bool
    type: ForloopFlowType
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:

        if isinstance(self.iterator, StaticTransform):
            iterator = self.iterator.to_dict()

        else:
            iterator = self.iterator.to_dict()

        skip_failures = self.skip_failures
        type = self.type.value

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "iterator": iterator,
                "skip_failures": skip_failures,
                "type": type,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()

        def _parse_iterator(data: object) -> Union[JavascriptTransform, StaticTransform]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_input_transform_type_0 = StaticTransform.from_dict(data)

                return componentsschemas_input_transform_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_input_transform_type_1 = JavascriptTransform.from_dict(data)

            return componentsschemas_input_transform_type_1

        iterator = _parse_iterator(d.pop("iterator"))

        skip_failures = d.pop("skip_failures")

        type = ForloopFlowType(d.pop("type"))

        forloop_flow = cls(
            iterator=iterator,
            skip_failures=skip_failures,
            type=type,
        )

        forloop_flow.additional_properties = d
        return forloop_flow

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
