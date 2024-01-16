from enum import Enum

class GraphqlType(str, Enum):
    GRAPHQL = "graphql"

    def __str__(self) -> str:
        return str(self.value)
