from enum import Enum


class QueuedJobRawFlowModulesItemSleepType0Type(str, Enum):
    JAVASCRIPT = "javascript"

    def __str__(self) -> str:
        return str(self.value)
