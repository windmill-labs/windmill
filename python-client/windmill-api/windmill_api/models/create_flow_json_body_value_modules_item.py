from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.create_flow_json_body_value_modules_item_input_transforms import (
    CreateFlowJsonBodyValueModulesItemInputTransforms,
)
from ..models.create_flow_json_body_value_modules_item_retry import CreateFlowJsonBodyValueModulesItemRetry
from ..models.create_flow_json_body_value_modules_item_sleep_type_0 import CreateFlowJsonBodyValueModulesItemSleepType0
from ..models.create_flow_json_body_value_modules_item_sleep_type_1 import CreateFlowJsonBodyValueModulesItemSleepType1
from ..models.create_flow_json_body_value_modules_item_stop_after_if import (
    CreateFlowJsonBodyValueModulesItemStopAfterIf,
)
from ..models.create_flow_json_body_value_modules_item_value_type_0 import CreateFlowJsonBodyValueModulesItemValueType0
from ..models.create_flow_json_body_value_modules_item_value_type_1 import CreateFlowJsonBodyValueModulesItemValueType1
from ..models.create_flow_json_body_value_modules_item_value_type_2 import CreateFlowJsonBodyValueModulesItemValueType2
from ..models.create_flow_json_body_value_modules_item_value_type_3 import CreateFlowJsonBodyValueModulesItemValueType3
from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateFlowJsonBodyValueModulesItem")


@attr.s(auto_attribs=True)
class CreateFlowJsonBodyValueModulesItem:
    """
    Attributes:
        input_transforms (CreateFlowJsonBodyValueModulesItemInputTransforms):
        value (Union[CreateFlowJsonBodyValueModulesItemValueType0, CreateFlowJsonBodyValueModulesItemValueType1,
            CreateFlowJsonBodyValueModulesItemValueType2, CreateFlowJsonBodyValueModulesItemValueType3]):
        stop_after_if (Union[Unset, CreateFlowJsonBodyValueModulesItemStopAfterIf]):
        sleep (Union[CreateFlowJsonBodyValueModulesItemSleepType0, CreateFlowJsonBodyValueModulesItemSleepType1,
            Unset]):
        summary (Union[Unset, str]):
        suspend (Union[Unset, int]):
        retry (Union[Unset, CreateFlowJsonBodyValueModulesItemRetry]):
    """

    input_transforms: CreateFlowJsonBodyValueModulesItemInputTransforms
    value: Union[
        CreateFlowJsonBodyValueModulesItemValueType0,
        CreateFlowJsonBodyValueModulesItemValueType1,
        CreateFlowJsonBodyValueModulesItemValueType2,
        CreateFlowJsonBodyValueModulesItemValueType3,
    ]
    stop_after_if: Union[Unset, CreateFlowJsonBodyValueModulesItemStopAfterIf] = UNSET
    sleep: Union[
        CreateFlowJsonBodyValueModulesItemSleepType0, CreateFlowJsonBodyValueModulesItemSleepType1, Unset
    ] = UNSET
    summary: Union[Unset, str] = UNSET
    suspend: Union[Unset, int] = UNSET
    retry: Union[Unset, CreateFlowJsonBodyValueModulesItemRetry] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        input_transforms = self.input_transforms.to_dict()

        if isinstance(self.value, CreateFlowJsonBodyValueModulesItemValueType0):
            value = self.value.to_dict()

        elif isinstance(self.value, CreateFlowJsonBodyValueModulesItemValueType1):
            value = self.value.to_dict()

        elif isinstance(self.value, CreateFlowJsonBodyValueModulesItemValueType2):
            value = self.value.to_dict()

        else:
            value = self.value.to_dict()

        stop_after_if: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.stop_after_if, Unset):
            stop_after_if = self.stop_after_if.to_dict()

        sleep: Union[Dict[str, Any], Unset]
        if isinstance(self.sleep, Unset):
            sleep = UNSET

        elif isinstance(self.sleep, CreateFlowJsonBodyValueModulesItemSleepType0):
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
        input_transforms = CreateFlowJsonBodyValueModulesItemInputTransforms.from_dict(d.pop("input_transforms"))

        def _parse_value(
            data: object,
        ) -> Union[
            CreateFlowJsonBodyValueModulesItemValueType0,
            CreateFlowJsonBodyValueModulesItemValueType1,
            CreateFlowJsonBodyValueModulesItemValueType2,
            CreateFlowJsonBodyValueModulesItemValueType3,
        ]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                value_type_0 = CreateFlowJsonBodyValueModulesItemValueType0.from_dict(data)

                return value_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                value_type_1 = CreateFlowJsonBodyValueModulesItemValueType1.from_dict(data)

                return value_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                value_type_2 = CreateFlowJsonBodyValueModulesItemValueType2.from_dict(data)

                return value_type_2
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            value_type_3 = CreateFlowJsonBodyValueModulesItemValueType3.from_dict(data)

            return value_type_3

        value = _parse_value(d.pop("value"))

        _stop_after_if = d.pop("stop_after_if", UNSET)
        stop_after_if: Union[Unset, CreateFlowJsonBodyValueModulesItemStopAfterIf]
        if isinstance(_stop_after_if, Unset):
            stop_after_if = UNSET
        else:
            stop_after_if = CreateFlowJsonBodyValueModulesItemStopAfterIf.from_dict(_stop_after_if)

        def _parse_sleep(
            data: object,
        ) -> Union[CreateFlowJsonBodyValueModulesItemSleepType0, CreateFlowJsonBodyValueModulesItemSleepType1, Unset]:
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                _sleep_type_0 = data
                sleep_type_0: Union[Unset, CreateFlowJsonBodyValueModulesItemSleepType0]
                if isinstance(_sleep_type_0, Unset):
                    sleep_type_0 = UNSET
                else:
                    sleep_type_0 = CreateFlowJsonBodyValueModulesItemSleepType0.from_dict(_sleep_type_0)

                return sleep_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            _sleep_type_1 = data
            sleep_type_1: Union[Unset, CreateFlowJsonBodyValueModulesItemSleepType1]
            if isinstance(_sleep_type_1, Unset):
                sleep_type_1 = UNSET
            else:
                sleep_type_1 = CreateFlowJsonBodyValueModulesItemSleepType1.from_dict(_sleep_type_1)

            return sleep_type_1

        sleep = _parse_sleep(d.pop("sleep", UNSET))

        summary = d.pop("summary", UNSET)

        suspend = d.pop("suspend", UNSET)

        _retry = d.pop("retry", UNSET)
        retry: Union[Unset, CreateFlowJsonBodyValueModulesItemRetry]
        if isinstance(_retry, Unset):
            retry = UNSET
        else:
            retry = CreateFlowJsonBodyValueModulesItemRetry.from_dict(_retry)

        create_flow_json_body_value_modules_item = cls(
            input_transforms=input_transforms,
            value=value,
            stop_after_if=stop_after_if,
            sleep=sleep,
            summary=summary,
            suspend=suspend,
            retry=retry,
        )

        create_flow_json_body_value_modules_item.additional_properties = d
        return create_flow_json_body_value_modules_item

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
