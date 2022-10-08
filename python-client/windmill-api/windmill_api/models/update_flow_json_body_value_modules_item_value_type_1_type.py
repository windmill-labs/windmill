from enum import Enum


class UpdateFlowJsonBodyValueModulesItemValueType1Type(str, Enum):
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)
