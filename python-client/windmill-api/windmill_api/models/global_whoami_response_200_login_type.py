from enum import Enum

class GlobalWhoamiResponse200LoginType(str, Enum):
    GITHUB = "github"
    PASSWORD = "password"

    def __str__(self) -> str:
        return str(self.value)
