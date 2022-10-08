from typing import Any, Dict, List, Type, TypeVar

import attr

from ..models.slack_token_bot import SlackTokenBot

T = TypeVar("T", bound="SlackToken")


@attr.s(auto_attribs=True)
class SlackToken:
    """
    Attributes:
        access_token (str):
        team_id (str):
        team_name (str):
        bot (SlackTokenBot):
    """

    access_token: str
    team_id: str
    team_name: str
    bot: SlackTokenBot
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        access_token = self.access_token
        team_id = self.team_id
        team_name = self.team_name
        bot = self.bot.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "access_token": access_token,
                "team_id": team_id,
                "team_name": team_name,
                "bot": bot,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        access_token = d.pop("access_token")

        team_id = d.pop("team_id")

        team_name = d.pop("team_name")

        bot = SlackTokenBot.from_dict(d.pop("bot"))

        slack_token = cls(
            access_token=access_token,
            team_id=team_id,
            team_name=team_name,
            bot=bot,
        )

        slack_token.additional_properties = d
        return slack_token

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
