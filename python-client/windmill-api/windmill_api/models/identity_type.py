from enum import Enum

class IdentityType(str, Enum):
    IDENTITY = "identity"

    def __str__(self) -> str:
        return str(self.value)
