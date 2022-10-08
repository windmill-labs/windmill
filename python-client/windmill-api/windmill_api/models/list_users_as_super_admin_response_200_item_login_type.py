from enum import Enum


class ListUsersAsSuperAdminResponse200ItemLoginType(str, Enum):
    PASSWORD = "password"
    GITHUB = "github"

    def __str__(self) -> str:
        return str(self.value)
