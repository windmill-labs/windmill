from enum import Enum


class OpenFlowValueFailureModuleValueType2Type(str, Enum):
    FORLOOPFLOW = "forloopflow"

    def __str__(self) -> str:
        return str(self.value)
