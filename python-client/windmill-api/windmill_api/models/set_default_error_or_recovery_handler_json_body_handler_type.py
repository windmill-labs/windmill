from enum import Enum

class SetDefaultErrorOrRecoveryHandlerJsonBodyHandlerType(str, Enum):
    ERROR = "error"
    RECOVERY = "recovery"

    def __str__(self) -> str:
        return str(self.value)
