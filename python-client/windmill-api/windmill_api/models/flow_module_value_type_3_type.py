from enum import Enum


class FlowModuleValueType3Type(str, Enum):
    FLOW = "flow"

    def __str__(self) -> str:
        return str(self.value)
