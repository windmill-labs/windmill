from enum import Enum

class AddGranularAclsKind(str, Enum):
    APP = "app"
    FLOW = "flow"
    FOLDER = "folder"
    GROUP = "group_"
    RAW_APP = "raw_app"
    RESOURCE = "resource"
    SCHEDULE = "schedule"
    SCRIPT = "script"
    VARIABLE = "variable"

    def __str__(self) -> str:
        return str(self.value)
