from enum import Enum


class GetFlowByPathResponse200ValueFailureModuleValueType1Type(str, Enum):
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)
