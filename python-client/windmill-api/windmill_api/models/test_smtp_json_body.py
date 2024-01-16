from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict

if TYPE_CHECKING:
  from ..models.test_smtp_json_body_smtp import TestSmtpJsonBodySmtp





T = TypeVar("T", bound="TestSmtpJsonBody")


@_attrs_define
class TestSmtpJsonBody:
    """ 
        Attributes:
            to (str):
            smtp (TestSmtpJsonBodySmtp):
     """

    to: str
    smtp: 'TestSmtpJsonBodySmtp'
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.test_smtp_json_body_smtp import TestSmtpJsonBodySmtp
        to = self.to
        smtp = self.smtp.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "to": to,
            "smtp": smtp,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.test_smtp_json_body_smtp import TestSmtpJsonBodySmtp
        d = src_dict.copy()
        to = d.pop("to")

        smtp = TestSmtpJsonBodySmtp.from_dict(d.pop("smtp"))




        test_smtp_json_body = cls(
            to=to,
            smtp=smtp,
        )

        test_smtp_json_body.additional_properties = d
        return test_smtp_json_body

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
