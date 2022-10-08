from typing import Any, Dict, List, Type, TypeVar

import attr

from ..models.connect_slack_callback_response_200_bot import ConnectSlackCallbackResponse200Bot

T = TypeVar("T", bound="ConnectSlackCallbackResponse200")


@attr.s(auto_attribs=True)
class ConnectSlackCallbackResponse200:
    """
    Attributes:
        access_token (str):
        team_id (str):
        team_name (str):
        bot (ConnectSlackCallbackResponse200Bot):
    """

    access_token: str
    team_id: str
    team_name: str
    bot: ConnectSlackCallbackResponse200Bot
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

        bot = ConnectSlackCallbackResponse200Bot.from_dict(d.pop("bot"))

        connect_slack_callback_response_200 = cls(
            access_token=access_token,
            team_id=team_id,
            team_name=team_name,
            bot=bot,
        )

        connect_slack_callback_response_200.additional_properties = d
        return connect_slack_callback_response_200

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
