"""SDK error types."""


class WorkerSdkError(Exception):
    """Base Python Worker SDK error."""


class ScriptExecutionError(WorkerSdkError):
    """Dynamic script execution failed."""


class ManagementRequestError(WorkerSdkError):
    """Management API request failed."""
