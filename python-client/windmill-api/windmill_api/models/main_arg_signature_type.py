from enum import Enum

class MainArgSignatureType(str, Enum):
    INVALID = "Invalid"
    VALID = "Valid"

    def __str__(self) -> str:
        return str(self.value)
