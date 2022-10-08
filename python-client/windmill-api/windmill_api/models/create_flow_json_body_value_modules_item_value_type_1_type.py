from enum import Enum


class CreateFlowJsonBodyValueModulesItemValueType1Type(str, Enum):
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)
