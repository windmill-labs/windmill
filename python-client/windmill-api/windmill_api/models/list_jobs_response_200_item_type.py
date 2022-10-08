from enum import Enum


class ListJobsResponse200ItemType(str, Enum):
    COMPLETEDJOB = "CompletedJob"
    QUEUEDJOB = "QueuedJob"

    def __str__(self) -> str:
        return str(self.value)
