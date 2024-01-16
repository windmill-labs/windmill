from enum import Enum

class FlowStatusFailureModuleType(str, Enum):
    FAILURE = "Failure"
    INPROGRESS = "InProgress"
    SUCCESS = "Success"
    WAITINGFOREVENTS = "WaitingForEvents"
    WAITINGFOREXECUTOR = "WaitingForExecutor"
    WAITINGFORPRIORSTEPS = "WaitingForPriorSteps"

    def __str__(self) -> str:
        return str(self.value)
