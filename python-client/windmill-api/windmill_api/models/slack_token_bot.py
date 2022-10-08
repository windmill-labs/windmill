from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="SlackTokenBot")


@attr.s(auto_attribs=True)
class SlackTokenBot:
    """
    Attributes:
        bot_access_token (Union[Unset, str]):
    """

    bot_access_token: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        bot_access_token = self.bot_access_token

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if bot_access_token is not UNSET:
            field_dict["bot_access_token"] = bot_access_token

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        bot_access_token = d.pop("bot_access_token", UNSET)

        slack_token_bot = cls(
            bot_access_token=bot_access_token,
        )

        slack_token_bot.additional_properties = d
        return slack_token_bot

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
