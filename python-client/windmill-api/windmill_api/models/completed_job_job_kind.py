from enum import Enum

class CompletedJobJobKind(str, Enum):
    APPDEPENDENCIES = "appdependencies"
    DEPENDENCIES = "dependencies"
    DEPLOYMENTCALLBACK = "deploymentcallback"
    FLOW = "flow"
    FLOWDEPENDENCIES = "flowdependencies"
    FLOWPREVIEW = "flowpreview"
    IDENTITY = "identity"
    PREVIEW = "preview"
    SCRIPT = "script"
    SCRIPT_HUB = "script_hub"
    SINGLESCRIPTFLOW = "singlescriptflow"

    def __str__(self) -> str:
        return str(self.value)
