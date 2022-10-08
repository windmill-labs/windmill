from enum import Enum


class UpdateFlowJsonBodyValueFailureModuleValueType2Type(str, Enum):
    FORLOOPFLOW = "forloopflow"

    def __str__(self) -> str:
        return str(self.value)
