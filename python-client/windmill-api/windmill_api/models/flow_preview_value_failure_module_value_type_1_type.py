from enum import Enum


class FlowPreviewValueFailureModuleValueType1Type(str, Enum):
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)
