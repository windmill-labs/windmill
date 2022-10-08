from enum import Enum


class GlobalUserInfoLoginType(str, Enum):
    PASSWORD = "password"
    GITHUB = "github"

    def __str__(self) -> str:
        return str(self.value)
