from enum import Enum


class CompletedJobRawFlowFailureModuleValueType3Type(str, Enum):
    FLOW = "flow"

    def __str__(self) -> str:
        return str(self.value)
