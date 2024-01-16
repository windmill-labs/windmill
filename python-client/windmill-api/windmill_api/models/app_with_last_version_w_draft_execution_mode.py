from enum import Enum

class AppWithLastVersionWDraftExecutionMode(str, Enum):
    ANONYMOUS = "anonymous"
    PUBLISHER = "publisher"
    VIEWER = "viewer"

    def __str__(self) -> str:
        return str(self.value)
