from enum import Enum


class GetCompletedJobResponse200RawFlowModulesItemValueType2Type(str, Enum):
    FORLOOPFLOW = "forloopflow"

    def __str__(self) -> str:
        return str(self.value)
