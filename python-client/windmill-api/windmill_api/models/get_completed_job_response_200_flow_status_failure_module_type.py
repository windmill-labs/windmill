from enum import Enum

class GetCompletedJobResponse200FlowStatusFailureModuleType(str, Enum):
    FAILURE = "Failure"
    INPROGRESS = "InProgress"
    SUCCESS = "Success"
    WAITINGFOREVENTS = "WaitingForEvents"
    WAITINGFOREXECUTOR = "WaitingForExecutor"
    WAITINGFORPRIORSTEPS = "WaitingForPriorSteps"

    def __str__(self) -> str:
        return str(self.value)
