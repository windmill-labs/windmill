from enum import Enum

class FlowModuleValue2Type7Type(str, Enum):
    GRAPHQL = "graphql"

    def __str__(self) -> str:
        return str(self.value)
