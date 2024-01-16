from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.edit_workspace_git_sync_config_json_body_git_sync_settings_item import EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem





T = TypeVar("T", bound="EditWorkspaceGitSyncConfigJsonBody")


@_attrs_define
class EditWorkspaceGitSyncConfigJsonBody:
    """ 
        Attributes:
            git_sync_settings (Union[Unset, List['EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem']]):
     """

    git_sync_settings: Union[Unset, List['EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem']] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.edit_workspace_git_sync_config_json_body_git_sync_settings_item import EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem
        git_sync_settings: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.git_sync_settings, Unset):
            git_sync_settings = []
            for git_sync_settings_item_data in self.git_sync_settings:
                git_sync_settings_item = git_sync_settings_item_data.to_dict()

                git_sync_settings.append(git_sync_settings_item)





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if git_sync_settings is not UNSET:
            field_dict["git_sync_settings"] = git_sync_settings

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.edit_workspace_git_sync_config_json_body_git_sync_settings_item import EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem
        d = src_dict.copy()
        git_sync_settings = []
        _git_sync_settings = d.pop("git_sync_settings", UNSET)
        for git_sync_settings_item_data in (_git_sync_settings or []):
            git_sync_settings_item = EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem.from_dict(git_sync_settings_item_data)



            git_sync_settings.append(git_sync_settings_item)


        edit_workspace_git_sync_config_json_body = cls(
            git_sync_settings=git_sync_settings,
        )

        edit_workspace_git_sync_config_json_body.additional_properties = d
        return edit_workspace_git_sync_config_json_body

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
