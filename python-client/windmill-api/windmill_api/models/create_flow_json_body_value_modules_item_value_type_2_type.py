from enum import Enum


class CreateFlowJsonBodyValueModulesItemValueType2Type(str, Enum):
    FORLOOPFLOW = "forloopflow"

    def __str__(self) -> str:
        return str(self.value)
