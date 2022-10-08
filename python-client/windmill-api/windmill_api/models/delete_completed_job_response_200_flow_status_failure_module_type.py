from enum import Enum


class DeleteCompletedJobResponse200FlowStatusFailureModuleType(str, Enum):
    WAITINGFORPRIORSTEPS = "WaitingForPriorSteps"
    WAITINGFOREVENT = "WaitingForEvent"
    WAITINGFOREXECUTOR = "WaitingForExecutor"
    INPROGRESS = "InProgress"
    SUCCESS = "Success"
    FAILURE = "Failure"

    def __str__(self) -> str:
        return str(self.value)
