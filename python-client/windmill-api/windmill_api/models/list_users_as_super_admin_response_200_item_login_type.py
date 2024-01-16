from enum import Enum

class ListUsersAsSuperAdminResponse200ItemLoginType(str, Enum):
    GITHUB = "github"
    PASSWORD = "password"

    def __str__(self) -> str:
        return str(self.value)
