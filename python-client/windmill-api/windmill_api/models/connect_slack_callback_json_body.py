from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="ConnectSlackCallbackJsonBody")


@attr.s(auto_attribs=True)
class ConnectSlackCallbackJsonBody:
    """
    Attributes:
        code (str):
        state (str):
    """

    code: str
    state: str
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        code = self.code
        state = self.state

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "code": code,
                "state": state,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        code = d.pop("code")

        state = d.pop("state")

        connect_slack_callback_json_body = cls(
            code=code,
            state=state,
        )

        connect_slack_callback_json_body.additional_properties = d
        return connect_slack_callback_json_body

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
