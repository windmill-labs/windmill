from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="RunSlackMessageTestJobJsonBody")


@_attrs_define
class RunSlackMessageTestJobJsonBody:
    """ 
        Attributes:
            hub_script_path (Union[Unset, str]):
            channel (Union[Unset, str]):
            test_msg (Union[Unset, str]):
     """

    hub_script_path: Union[Unset, str] = UNSET
    channel: Union[Unset, str] = UNSET
    test_msg: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        hub_script_path = self.hub_script_path
        channel = self.channel
        test_msg = self.test_msg

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if hub_script_path is not UNSET:
            field_dict["hub_script_path"] = hub_script_path
        if channel is not UNSET:
            field_dict["channel"] = channel
        if test_msg is not UNSET:
            field_dict["test_msg"] = test_msg

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        hub_script_path = d.pop("hub_script_path", UNSET)

        channel = d.pop("channel", UNSET)

        test_msg = d.pop("test_msg", UNSET)

        run_slack_message_test_job_json_body = cls(
            hub_script_path=hub_script_path,
            channel=channel,
            test_msg=test_msg,
        )

        run_slack_message_test_job_json_body.additional_properties = d
        return run_slack_message_test_job_json_body

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
