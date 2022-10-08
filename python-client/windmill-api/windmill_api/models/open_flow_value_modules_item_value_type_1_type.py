from enum import Enum


class OpenFlowValueModulesItemValueType1Type(str, Enum):
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)
