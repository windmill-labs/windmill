from enum import Enum


class CompletedJobRawFlowModulesItemValueType3Type(str, Enum):
    FLOW = "flow"

    def __str__(self) -> str:
        return str(self.value)
