from enum import Enum

class GlobalUserInfoLoginType(str, Enum):
    GITHUB = "github"
    PASSWORD = "password"

    def __str__(self) -> str:
        return str(self.value)
