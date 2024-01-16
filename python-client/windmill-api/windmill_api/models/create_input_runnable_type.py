from enum import Enum

class CreateInputRunnableType(str, Enum):
    FLOWPATH = "FlowPath"
    SCRIPTHASH = "ScriptHash"
    SCRIPTPATH = "ScriptPath"

    def __str__(self) -> str:
        return str(self.value)
