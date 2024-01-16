from enum import Enum

class NewScriptKind(str, Enum):
    APPROVAL = "approval"
    COMMAND = "command"
    FAILURE = "failure"
    SCRIPT = "script"
    TRIGGER = "trigger"

    def __str__(self) -> str:
        return str(self.value)
