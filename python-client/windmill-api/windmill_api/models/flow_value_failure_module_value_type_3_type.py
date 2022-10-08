from enum import Enum


class FlowValueFailureModuleValueType3Type(str, Enum):
    FLOW = "flow"

    def __str__(self) -> str:
        return str(self.value)
