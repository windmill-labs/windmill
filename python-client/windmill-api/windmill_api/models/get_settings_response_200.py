from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="GetSettingsResponse200")


@attr.s(auto_attribs=True)
class GetSettingsResponse200:
    """
    Attributes:
        workspace_id (Union[Unset, str]):
        slack_name (Union[Unset, str]):
        slack_team_id (Union[Unset, str]):
        slack_command_script (Union[Unset, str]):
    """

    workspace_id: Union[Unset, str] = UNSET
    slack_name: Union[Unset, str] = UNSET
    slack_team_id: Union[Unset, str] = UNSET
    slack_command_script: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        workspace_id = self.workspace_id
        slack_name = self.slack_name
        slack_team_id = self.slack_team_id
        slack_command_script = self.slack_command_script

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if slack_name is not UNSET:
            field_dict["slack_name"] = slack_name
        if slack_team_id is not UNSET:
            field_dict["slack_team_id"] = slack_team_id
        if slack_command_script is not UNSET:
            field_dict["slack_command_script"] = slack_command_script

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        workspace_id = d.pop("workspace_id", UNSET)

        slack_name = d.pop("slack_name", UNSET)

        slack_team_id = d.pop("slack_team_id", UNSET)

        slack_command_script = d.pop("slack_command_script", UNSET)

        get_settings_response_200 = cls(
            workspace_id=workspace_id,
            slack_name=slack_name,
            slack_team_id=slack_team_id,
            slack_command_script=slack_command_script,
        )

        get_settings_response_200.additional_properties = d
        return get_settings_response_200

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
