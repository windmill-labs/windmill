from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem")


@_attrs_define
class EditWorkspaceGitSyncConfigJsonBodyGitSyncSettingsItem:
    """ 
        Attributes:
            script_path (str):
            git_repo_resource_path (str):
            use_individual_branch (Union[Unset, bool]):
     """

    script_path: str
    git_repo_resource_path: str
    use_individual_branch: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        script_path = self.script_path
        git_repo_resource_path = self.git_repo_resource_path
        use_individual_branch = self.use_individual_branch

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "script_path": script_path,
            "git_repo_resource_path": git_repo_resource_path,
        })
        if use_individual_branch is not UNSET:
            field_dict["use_individual_branch"] = use_individual_branch

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        script_path = d.pop("script_path")

        git_repo_resource_path = d.pop("git_repo_resource_path")

        use_individual_branch = d.pop("use_individual_branch", UNSET)

        edit_workspace_git_sync_config_json_body_git_sync_settings_item = cls(
            script_path=script_path,
            git_repo_resource_path=git_repo_resource_path,
            use_individual_branch=use_individual_branch,
        )

        edit_workspace_git_sync_config_json_body_git_sync_settings_item.additional_properties = d
        return edit_workspace_git_sync_config_json_body_git_sync_settings_item

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
