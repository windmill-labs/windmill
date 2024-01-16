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
  from ..models.get_settings_response_200_large_file_storage import GetSettingsResponse200LargeFileStorage
  from ..models.get_settings_response_200_error_handler_extra_args import GetSettingsResponse200ErrorHandlerExtraArgs
  from ..models.get_settings_response_200_git_sync_item import GetSettingsResponse200GitSyncItem





T = TypeVar("T", bound="GetSettingsResponse200")


@_attrs_define
class GetSettingsResponse200:
    """ 
        Attributes:
            code_completion_enabled (bool):
            workspace_id (Union[Unset, str]):
            slack_name (Union[Unset, str]):
            slack_team_id (Union[Unset, str]):
            slack_command_script (Union[Unset, str]):
            auto_invite_domain (Union[Unset, str]):
            auto_invite_operator (Union[Unset, bool]):
            plan (Union[Unset, str]):
            customer_id (Union[Unset, str]):
            webhook (Union[Unset, str]):
            deploy_to (Union[Unset, str]):
            openai_resource_path (Union[Unset, str]):
            error_handler (Union[Unset, str]):
            error_handler_extra_args (Union[Unset, GetSettingsResponse200ErrorHandlerExtraArgs]):
            error_handler_muted_on_cancel (Union[Unset, bool]):
            large_file_storage (Union[Unset, GetSettingsResponse200LargeFileStorage]):
            git_sync (Union[Unset, List['GetSettingsResponse200GitSyncItem']]):
     """

    code_completion_enabled: bool
    workspace_id: Union[Unset, str] = UNSET
    slack_name: Union[Unset, str] = UNSET
    slack_team_id: Union[Unset, str] = UNSET
    slack_command_script: Union[Unset, str] = UNSET
    auto_invite_domain: Union[Unset, str] = UNSET
    auto_invite_operator: Union[Unset, bool] = UNSET
    plan: Union[Unset, str] = UNSET
    customer_id: Union[Unset, str] = UNSET
    webhook: Union[Unset, str] = UNSET
    deploy_to: Union[Unset, str] = UNSET
    openai_resource_path: Union[Unset, str] = UNSET
    error_handler: Union[Unset, str] = UNSET
    error_handler_extra_args: Union[Unset, 'GetSettingsResponse200ErrorHandlerExtraArgs'] = UNSET
    error_handler_muted_on_cancel: Union[Unset, bool] = UNSET
    large_file_storage: Union[Unset, 'GetSettingsResponse200LargeFileStorage'] = UNSET
    git_sync: Union[Unset, List['GetSettingsResponse200GitSyncItem']] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_settings_response_200_large_file_storage import GetSettingsResponse200LargeFileStorage
        from ..models.get_settings_response_200_error_handler_extra_args import GetSettingsResponse200ErrorHandlerExtraArgs
        from ..models.get_settings_response_200_git_sync_item import GetSettingsResponse200GitSyncItem
        code_completion_enabled = self.code_completion_enabled
        workspace_id = self.workspace_id
        slack_name = self.slack_name
        slack_team_id = self.slack_team_id
        slack_command_script = self.slack_command_script
        auto_invite_domain = self.auto_invite_domain
        auto_invite_operator = self.auto_invite_operator
        plan = self.plan
        customer_id = self.customer_id
        webhook = self.webhook
        deploy_to = self.deploy_to
        openai_resource_path = self.openai_resource_path
        error_handler = self.error_handler
        error_handler_extra_args: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.error_handler_extra_args, Unset):
            error_handler_extra_args = self.error_handler_extra_args.to_dict()

        error_handler_muted_on_cancel = self.error_handler_muted_on_cancel
        large_file_storage: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.large_file_storage, Unset):
            large_file_storage = self.large_file_storage.to_dict()

        git_sync: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.git_sync, Unset):
            git_sync = []
            for git_sync_item_data in self.git_sync:
                git_sync_item = git_sync_item_data.to_dict()

                git_sync.append(git_sync_item)





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "code_completion_enabled": code_completion_enabled,
        })
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if slack_name is not UNSET:
            field_dict["slack_name"] = slack_name
        if slack_team_id is not UNSET:
            field_dict["slack_team_id"] = slack_team_id
        if slack_command_script is not UNSET:
            field_dict["slack_command_script"] = slack_command_script
        if auto_invite_domain is not UNSET:
            field_dict["auto_invite_domain"] = auto_invite_domain
        if auto_invite_operator is not UNSET:
            field_dict["auto_invite_operator"] = auto_invite_operator
        if plan is not UNSET:
            field_dict["plan"] = plan
        if customer_id is not UNSET:
            field_dict["customer_id"] = customer_id
        if webhook is not UNSET:
            field_dict["webhook"] = webhook
        if deploy_to is not UNSET:
            field_dict["deploy_to"] = deploy_to
        if openai_resource_path is not UNSET:
            field_dict["openai_resource_path"] = openai_resource_path
        if error_handler is not UNSET:
            field_dict["error_handler"] = error_handler
        if error_handler_extra_args is not UNSET:
            field_dict["error_handler_extra_args"] = error_handler_extra_args
        if error_handler_muted_on_cancel is not UNSET:
            field_dict["error_handler_muted_on_cancel"] = error_handler_muted_on_cancel
        if large_file_storage is not UNSET:
            field_dict["large_file_storage"] = large_file_storage
        if git_sync is not UNSET:
            field_dict["git_sync"] = git_sync

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_settings_response_200_large_file_storage import GetSettingsResponse200LargeFileStorage
        from ..models.get_settings_response_200_error_handler_extra_args import GetSettingsResponse200ErrorHandlerExtraArgs
        from ..models.get_settings_response_200_git_sync_item import GetSettingsResponse200GitSyncItem
        d = src_dict.copy()
        code_completion_enabled = d.pop("code_completion_enabled")

        workspace_id = d.pop("workspace_id", UNSET)

        slack_name = d.pop("slack_name", UNSET)

        slack_team_id = d.pop("slack_team_id", UNSET)

        slack_command_script = d.pop("slack_command_script", UNSET)

        auto_invite_domain = d.pop("auto_invite_domain", UNSET)

        auto_invite_operator = d.pop("auto_invite_operator", UNSET)

        plan = d.pop("plan", UNSET)

        customer_id = d.pop("customer_id", UNSET)

        webhook = d.pop("webhook", UNSET)

        deploy_to = d.pop("deploy_to", UNSET)

        openai_resource_path = d.pop("openai_resource_path", UNSET)

        error_handler = d.pop("error_handler", UNSET)

        _error_handler_extra_args = d.pop("error_handler_extra_args", UNSET)
        error_handler_extra_args: Union[Unset, GetSettingsResponse200ErrorHandlerExtraArgs]
        if isinstance(_error_handler_extra_args,  Unset):
            error_handler_extra_args = UNSET
        else:
            error_handler_extra_args = GetSettingsResponse200ErrorHandlerExtraArgs.from_dict(_error_handler_extra_args)




        error_handler_muted_on_cancel = d.pop("error_handler_muted_on_cancel", UNSET)

        _large_file_storage = d.pop("large_file_storage", UNSET)
        large_file_storage: Union[Unset, GetSettingsResponse200LargeFileStorage]
        if isinstance(_large_file_storage,  Unset):
            large_file_storage = UNSET
        else:
            large_file_storage = GetSettingsResponse200LargeFileStorage.from_dict(_large_file_storage)




        git_sync = []
        _git_sync = d.pop("git_sync", UNSET)
        for git_sync_item_data in (_git_sync or []):
            git_sync_item = GetSettingsResponse200GitSyncItem.from_dict(git_sync_item_data)



            git_sync.append(git_sync_item)


        get_settings_response_200 = cls(
            code_completion_enabled=code_completion_enabled,
            workspace_id=workspace_id,
            slack_name=slack_name,
            slack_team_id=slack_team_id,
            slack_command_script=slack_command_script,
            auto_invite_domain=auto_invite_domain,
            auto_invite_operator=auto_invite_operator,
            plan=plan,
            customer_id=customer_id,
            webhook=webhook,
            deploy_to=deploy_to,
            openai_resource_path=openai_resource_path,
            error_handler=error_handler,
            error_handler_extra_args=error_handler_extra_args,
            error_handler_muted_on_cancel=error_handler_muted_on_cancel,
            large_file_storage=large_file_storage,
            git_sync=git_sync,
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
