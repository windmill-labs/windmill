import datetime
from typing import Any, Dict, List, Type, TypeVar, Union

import attr
from dateutil.parser import isoparse

from ..models.list_audit_logs_response_200_item_action_kind import ListAuditLogsResponse200ItemActionKind
from ..models.list_audit_logs_response_200_item_operation import ListAuditLogsResponse200ItemOperation
from ..models.list_audit_logs_response_200_item_parameters import ListAuditLogsResponse200ItemParameters
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListAuditLogsResponse200Item")


@attr.s(auto_attribs=True)
class ListAuditLogsResponse200Item:
    """
    Attributes:
        id (int):
        timestamp (datetime.datetime):
        username (str):
        operation (ListAuditLogsResponse200ItemOperation):
        action_kind (ListAuditLogsResponse200ItemActionKind):
        resource (Union[Unset, str]):
        parameters (Union[Unset, ListAuditLogsResponse200ItemParameters]):
    """

    id: int
    timestamp: datetime.datetime
    username: str
    operation: ListAuditLogsResponse200ItemOperation
    action_kind: ListAuditLogsResponse200ItemActionKind
    resource: Union[Unset, str] = UNSET
    parameters: Union[Unset, ListAuditLogsResponse200ItemParameters] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        id = self.id
        timestamp = self.timestamp.isoformat()

        username = self.username
        operation = self.operation.value

        action_kind = self.action_kind.value

        resource = self.resource
        parameters: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.parameters, Unset):
            parameters = self.parameters.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "id": id,
                "timestamp": timestamp,
                "username": username,
                "operation": operation,
                "action_kind": action_kind,
            }
        )
        if resource is not UNSET:
            field_dict["resource"] = resource
        if parameters is not UNSET:
            field_dict["parameters"] = parameters

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        id = d.pop("id")

        timestamp = isoparse(d.pop("timestamp"))

        username = d.pop("username")

        operation = ListAuditLogsResponse200ItemOperation(d.pop("operation"))

        action_kind = ListAuditLogsResponse200ItemActionKind(d.pop("action_kind"))

        resource = d.pop("resource", UNSET)

        _parameters = d.pop("parameters", UNSET)
        parameters: Union[Unset, ListAuditLogsResponse200ItemParameters]
        if isinstance(_parameters, Unset):
            parameters = UNSET
        else:
            parameters = ListAuditLogsResponse200ItemParameters.from_dict(_parameters)

        list_audit_logs_response_200_item = cls(
            id=id,
            timestamp=timestamp,
            username=username,
            operation=operation,
            action_kind=action_kind,
            resource=resource,
            parameters=parameters,
        )

        list_audit_logs_response_200_item.additional_properties = d
        return list_audit_logs_response_200_item

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
