from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset







T = TypeVar("T", bound="TestSmtpJsonBodySmtp")


@_attrs_define
class TestSmtpJsonBodySmtp:
    """ 
        Attributes:
            host (str):
            username (str):
            password (str):
            port (int):
            from_ (str):
            tls_implicit (bool):
     """

    host: str
    username: str
    password: str
    port: int
    from_: str
    tls_implicit: bool
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        host = self.host
        username = self.username
        password = self.password
        port = self.port
        from_ = self.from_
        tls_implicit = self.tls_implicit

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "host": host,
            "username": username,
            "password": password,
            "port": port,
            "from": from_,
            "tls_implicit": tls_implicit,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        host = d.pop("host")

        username = d.pop("username")

        password = d.pop("password")

        port = d.pop("port")

        from_ = d.pop("from")

        tls_implicit = d.pop("tls_implicit")

        test_smtp_json_body_smtp = cls(
            host=host,
            username=username,
            password=password,
            port=port,
            from_=from_,
            tls_implicit=tls_implicit,
        )

        test_smtp_json_body_smtp.additional_properties = d
        return test_smtp_json_body_smtp

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
