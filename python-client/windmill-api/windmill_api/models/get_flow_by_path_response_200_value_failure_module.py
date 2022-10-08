from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.get_flow_by_path_response_200_value_failure_module_input_transforms import (
    GetFlowByPathResponse200ValueFailureModuleInputTransforms,
)
from ..models.get_flow_by_path_response_200_value_failure_module_retry import (
    GetFlowByPathResponse200ValueFailureModuleRetry,
)
from ..models.get_flow_by_path_response_200_value_failure_module_sleep_type_0 import (
    GetFlowByPathResponse200ValueFailureModuleSleepType0,
)
from ..models.get_flow_by_path_response_200_value_failure_module_sleep_type_1 import (
    GetFlowByPathResponse200ValueFailureModuleSleepType1,
)
from ..models.get_flow_by_path_response_200_value_failure_module_stop_after_if import (
    GetFlowByPathResponse200ValueFailureModuleStopAfterIf,
)
from ..models.get_flow_by_path_response_200_value_failure_module_value_type_0 import (
    GetFlowByPathResponse200ValueFailureModuleValueType0,
)
from ..models.get_flow_by_path_response_200_value_failure_module_value_type_1 import (
    GetFlowByPathResponse200ValueFailureModuleValueType1,
)
from ..models.get_flow_by_path_response_200_value_failure_module_value_type_2 import (
    GetFlowByPathResponse200ValueFailureModuleValueType2,
)
from ..models.get_flow_by_path_response_200_value_failure_module_value_type_3 import (
    GetFlowByPathResponse200ValueFailureModuleValueType3,
)
from ..types import UNSET, Unset

T = TypeVar("T", bound="GetFlowByPathResponse200ValueFailureModule")


@attr.s(auto_attribs=True)
class GetFlowByPathResponse200ValueFailureModule:
    """
    Attributes:
        input_transforms (GetFlowByPathResponse200ValueFailureModuleInputTransforms):
        value (Union[GetFlowByPathResponse200ValueFailureModuleValueType0,
            GetFlowByPathResponse200ValueFailureModuleValueType1, GetFlowByPathResponse200ValueFailureModuleValueType2,
            GetFlowByPathResponse200ValueFailureModuleValueType3]):
        stop_after_if (Union[Unset, GetFlowByPathResponse200ValueFailureModuleStopAfterIf]):
        sleep (Union[GetFlowByPathResponse200ValueFailureModuleSleepType0,
            GetFlowByPathResponse200ValueFailureModuleSleepType1, Unset]):
        summary (Union[Unset, str]):
        suspend (Union[Unset, int]):
        retry (Union[Unset, GetFlowByPathResponse200ValueFailureModuleRetry]):
    """

    input_transforms: GetFlowByPathResponse200ValueFailureModuleInputTransforms
    value: Union[
        GetFlowByPathResponse200ValueFailureModuleValueType0,
        GetFlowByPathResponse200ValueFailureModuleValueType1,
        GetFlowByPathResponse200ValueFailureModuleValueType2,
        GetFlowByPathResponse200ValueFailureModuleValueType3,
    ]
    stop_after_if: Union[Unset, GetFlowByPathResponse200ValueFailureModuleStopAfterIf] = UNSET
    sleep: Union[
        GetFlowByPathResponse200ValueFailureModuleSleepType0,
        GetFlowByPathResponse200ValueFailureModuleSleepType1,
        Unset,
    ] = UNSET
    summary: Union[Unset, str] = UNSET
    suspend: Union[Unset, int] = UNSET
    retry: Union[Unset, GetFlowByPathResponse200ValueFailureModuleRetry] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        input_transforms = self.input_transforms.to_dict()

        if isinstance(self.value, GetFlowByPathResponse200ValueFailureModuleValueType0):
            value = self.value.to_dict()

        elif isinstance(self.value, GetFlowByPathResponse200ValueFailureModuleValueType1):
            value = self.value.to_dict()

        elif isinstance(self.value, GetFlowByPathResponse200ValueFailureModuleValueType2):
            value = self.value.to_dict()

        else:
            value = self.value.to_dict()

        stop_after_if: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.stop_after_if, Unset):
            stop_after_if = self.stop_after_if.to_dict()

        sleep: Union[Dict[str, Any], Unset]
        if isinstance(self.sleep, Unset):
            sleep = UNSET

        elif isinstance(self.sleep, GetFlowByPathResponse200ValueFailureModuleSleepType0):
            sleep = UNSET
            if not isinstance(self.sleep, Unset):
                sleep = self.sleep.to_dict()

        else:
            sleep = UNSET
            if not isinstance(self.sleep, Unset):
                sleep = self.sleep.to_dict()

        summary = self.summary
        suspend = self.suspend
        retry: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.retry, Unset):
            retry = self.retry.to_dict()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "input_transforms": input_transforms,
                "value": value,
            }
        )
        if stop_after_if is not UNSET:
            field_dict["stop_after_if"] = stop_after_if
        if sleep is not UNSET:
            field_dict["sleep"] = sleep
        if summary is not UNSET:
            field_dict["summary"] = summary
        if suspend is not UNSET:
            field_dict["suspend"] = suspend
        if retry is not UNSET:
            field_dict["retry"] = retry

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        input_transforms = GetFlowByPathResponse200ValueFailureModuleInputTransforms.from_dict(
            d.pop("input_transforms")
        )

        def _parse_value(
            data: object,
        ) -> Union[
            GetFlowByPathResponse200ValueFailureModuleValueType0,
            GetFlowByPathResponse200ValueFailureModuleValueType1,
            GetFlowByPathResponse200ValueFailureModuleValueType2,
            GetFlowByPathResponse200ValueFailureModuleValueType3,
        ]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                value_type_0 = GetFlowByPathResponse200ValueFailureModuleValueType0.from_dict(data)

                return value_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                value_type_1 = GetFlowByPathResponse200ValueFailureModuleValueType1.from_dict(data)

                return value_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                value_type_2 = GetFlowByPathResponse200ValueFailureModuleValueType2.from_dict(data)

                return value_type_2
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            value_type_3 = GetFlowByPathResponse200ValueFailureModuleValueType3.from_dict(data)

            return value_type_3

        value = _parse_value(d.pop("value"))

        _stop_after_if = d.pop("stop_after_if", UNSET)
        stop_after_if: Union[Unset, GetFlowByPathResponse200ValueFailureModuleStopAfterIf]
        if isinstance(_stop_after_if, Unset):
            stop_after_if = UNSET
        else:
            stop_after_if = GetFlowByPathResponse200ValueFailureModuleStopAfterIf.from_dict(_stop_after_if)

        def _parse_sleep(
            data: object,
        ) -> Union[
            GetFlowByPathResponse200ValueFailureModuleSleepType0,
            GetFlowByPathResponse200ValueFailureModuleSleepType1,
            Unset,
        ]:
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                _sleep_type_0 = data
                sleep_type_0: Union[Unset, GetFlowByPathResponse200ValueFailureModuleSleepType0]
                if isinstance(_sleep_type_0, Unset):
                    sleep_type_0 = UNSET
                else:
                    sleep_type_0 = GetFlowByPathResponse200ValueFailureModuleSleepType0.from_dict(_sleep_type_0)

                return sleep_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            _sleep_type_1 = data
            sleep_type_1: Union[Unset, GetFlowByPathResponse200ValueFailureModuleSleepType1]
            if isinstance(_sleep_type_1, Unset):
                sleep_type_1 = UNSET
            else:
                sleep_type_1 = GetFlowByPathResponse200ValueFailureModuleSleepType1.from_dict(_sleep_type_1)

            return sleep_type_1

        sleep = _parse_sleep(d.pop("sleep", UNSET))

        summary = d.pop("summary", UNSET)

        suspend = d.pop("suspend", UNSET)

        _retry = d.pop("retry", UNSET)
        retry: Union[Unset, GetFlowByPathResponse200ValueFailureModuleRetry]
        if isinstance(_retry, Unset):
            retry = UNSET
        else:
            retry = GetFlowByPathResponse200ValueFailureModuleRetry.from_dict(_retry)

        get_flow_by_path_response_200_value_failure_module = cls(
            input_transforms=input_transforms,
            value=value,
            stop_after_if=stop_after_if,
            sleep=sleep,
            summary=summary,
            suspend=suspend,
            retry=retry,
        )

        get_flow_by_path_response_200_value_failure_module.additional_properties = d
        return get_flow_by_path_response_200_value_failure_module

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
