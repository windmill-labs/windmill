from __future__ import annotations

import atexit
import datetime as dt
import functools
import logging
import os
import random
import time
import warnings
from json import JSONDecodeError
from typing import Dict, Any, Union, Literal

import httpx

_client: "Windmill | None" = None

logger = logging.getLogger("windmill_client")

JobStatus = Literal["RUNNING", "WAITING", "COMPLETED"]


class Windmill:
    def __init__(self, base_url=None, token=None, workspace=None):
        base = base_url or os.environ.get("BASE_INTERNAL_URL")

        self.base_url = f"{base}/api"
        self.token = token or os.environ.get("WM_TOKEN")
        self.headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.token}",
        }
        self.client = self.get_client()
        self.workspace = workspace or os.environ.get("WM_WORKSPACE")
        self.path = os.environ.get("WM_JOB_PATH")

        assert self.workspace, f"workspace required as an argument or as WM_WORKSPACE environment variable"

    def get_client(self) -> httpx.Client:
        return httpx.Client(
            base_url=self.base_url,
            headers=self.headers,
        )

    def get(self, endpoint, raise_for_status=True, **kwargs) -> httpx.Response:
        endpoint = endpoint.lstrip("/")
        resp = self.client.get(f"/{endpoint}", **kwargs)
        if raise_for_status:
            try:
                resp.raise_for_status()
            except httpx.HTTPStatusError as err:
                error = f"{err.request.url}: {err.response.status_code}, {err.response.text}"
                logger.error(error)
                raise Exception(error)
        return resp

    def post(self, endpoint, raise_for_status=True, **kwargs) -> httpx.Response:
        endpoint = endpoint.lstrip("/")
        resp = self.client.post(f"/{endpoint}", **kwargs)
        if raise_for_status:
            try:
                resp.raise_for_status()
            except httpx.HTTPStatusError as err:
                error = f"{err.request.url}: {err.response.status_code}, {err.response.text}"
                logger.error(error)
                raise Exception(error)
        return resp

    def create_token(self, duration=dt.timedelta(days=1)) -> str:
        endpoint = "/users/tokens/create"
        payload = {
            "label": f"refresh {time.time()}",
            "expiration": (dt.datetime.now() + duration).strftime("%Y-%m-%dT%H:%M:%SZ"),
        }
        return self.post(endpoint, json=payload).text

    def run_script_async(
        self,
        path: str = None,
        hash_: str = None,
        args: dict = None,
        scheduled_in_secs: int = None,
    ) -> str:
        """Create a script job and return its job id."""
        assert not (path and hash_), "path and hash_ are mutually exclusive"
        args = args or {}
        params = {"scheduled_in_secs": scheduled_in_secs} if scheduled_in_secs else {}
        if path:
            endpoint = f"/w/{self.workspace}/jobs/run/p/{path}"
        elif hash_:
            endpoint = f"/w/{self.workspace}/jobs/run/h/{hash_}"
        else:
            raise Exception("path or hash_ must be provided")
        return self.post(endpoint, json=args, params=params).text

    def run_flow_async(
        self,
        path: str,
        args: dict = None,
        scheduled_in_secs: int = None,
    ) -> str:
        """Create a flow job and return its job id."""
        args = args or {}
        params = {"scheduled_in_secs": scheduled_in_secs} if scheduled_in_secs else {}
        if path:
            endpoint = f"/w/{self.workspace}/jobs/run/f/{path}"
        else:
            raise Exception("path must be provided")
        return self.post(endpoint, json=args, params=params).text

    def run_script(
        self,
        path: str = None,
        hash_: str = None,
        args: dict = None,
        timeout: dt.timedelta | int | float = None,
        verbose: bool = False,
        cleanup: bool = True,
        assert_result_is_not_none: bool = True,
    ) -> Any:
        """Run script synchronously and return its result."""
        args = args or {}

        if verbose:
            logger.info(f"running `{path}` synchronously with {args = }")

        if isinstance(timeout, dt.timedelta):
            timeout = timeout.total_seconds

        start_time = time.time()

        job_id = self.run_script_async(path=path, hash_=hash_, args=args)

        def cancel_job():
            logger.warning(f"cancelling job: {job_id}")
            self.post(
                f"/w/{self.workspace}/jobs_u/queue/cancel/{job_id}",
                json={"reason": "parent script cancelled"},
            ).raise_for_status()

        if cleanup:
            atexit.register(cancel_job)

        while True:
            job = self.get_job(job_id)

            if timeout and ((time.time() - start_time) > timeout):
                msg = "reached timeout"
                logger.warning(msg)
                self.post(
                    f"/w/{self.workspace}/jobs_u/queue/cancel/{job_id}",
                    json={"reason": msg},
                )
                raise TimeoutError(msg)

            result = job.get("result")
            canceled, canceled_reason = job.get("canceled"), job.get("canceled_reason")
            success = job.get("success")
            job_type = job.get("type", "")
            completed = job_type.lower() == "completedjob"

            if cleanup and completed:
                atexit.unregister(cancel_job)

            if completed:
                if success:
                    if assert_result_is_not_none and result is None:
                        raise Exception(f"result is None for {job_id = }")
                    return result
                else:
                    if canceled:
                        raise Exception(f"job canceled: {canceled_reason}")
                    else:
                        error = result.get("error")
                        raise Exception(f"job failed: {error}")

            if verbose:
                logger.info(f"sleeping 0.5 seconds for {job_id = }")

            time.sleep(0.5)

    def cancel_running(self) -> dict:
        """Cancel currently running executions of the same script."""
        logger.info("canceling running executions of this script")

        jobs = self.get(
            f"/w/{self.workspace}/jobs/list",
            params={
                "running": "true",
                "script_path_exact": self.path,
            },
        ).json()

        current_job_id = os.environ.get("WM_JOB_ID")

        logger.debug(f"{current_job_id = }")

        job_ids = [j["id"] for j in jobs if j["id"] != current_job_id]

        if job_ids:
            logger.info(f"cancelling the following job ids: {job_ids}")
        else:
            logger.info("no previous executions to cancel")

        result = {}

        for id_ in job_ids:
            result[id_] = self.post(
                f"/w/{self.workspace}/jobs_u/queue/cancel/{id_}",
                json={"reason": "killed by `cancel_running` method"},
            )

        return result

    def get_job(self, job_id: str) -> dict:
        return self.get(f"/w/{self.workspace}/jobs_u/get/{job_id}").json()

    def get_job_status(self, job_id: str) -> JobStatus:
        job = self.get_job(job_id)
        job_type = job.get("type", "")
        assert job_type, f"{job} is not a valid job"
        if job_type.lower() == "completedjob":
            return "COMPLETED"
        additional_properties = job.get("additional_properties", {})
        if "running" not in additional_properties:
            raise Exception(f"{job_id} is not running")
        if additional_properties.get("running"):
            return "RUNNING"
        return "WAITING"

    def get_result(
        self,
        job_id: str,
        assert_result_is_not_none: bool = True,
    ) -> Any:
        result = self.get(f"/w/{self.workspace}/jobs_u/completed/get_result/{job_id}")
        result_text = result.text
        if assert_result_is_not_none and result_text is None:
            raise Exception(f"result is None for {job_id = }")
        try:
            return result.json()
        except JSONDecodeError:
            return result_text

    def get_variable(self, path: str) -> str:
        """Get variable from Windmill"""
        return self.get(f"/w/{self.workspace}/variables/get_value/{path}").json()

    def set_variable(self, path: str, value: str) -> None:
        """Set variable from Windmill"""
        # check if variable exists
        r = self.get(f"/w/{self.workspace}/variables/get/{path}", raise_for_status=False)
        if r.status_code == 404:
            # create variable
            self.post(
                f"/w/{self.workspace}/variables/create",
                json={
                    "path": path,
                    "value": value,
                    "is_secret": False,
                    "description": "",
                },
            )
        else:
            # update variable
            self.post(
                f"/w/{self.workspace}/variables/update/{path}",
                json={"value": value},
            )

    def get_resource(
        self,
        path: str,
        none_if_undefined: bool = False,
    ) -> str | dict | None:
        """Get resource from Windmill"""
        try:
            return self.get(f"/w/{self.workspace}/resources/get_value_interpolated/{path}").json()
        except Exception as e:
            if none_if_undefined:
                return None
            logger.error(e)
            raise e

    def set_resource(
        self,
        value: Any,
        path: str,
        resource_type: str,
    ):
        # check if resource exists
        r = self.get(f"/w/{self.workspace}/resources/get/{path}", raise_for_status=False)
        if r.status_code == 404:
            # create resource
            self.post(
                f"/w/{self.workspace}/resources/create",
                json={
                    "path": path,
                    "value": value,
                    "resource_type": resource_type,
                },
            )
        else:
            # update resource
            self.post(
                f"/w/{self.workspace}/resources/update_value/{path}",
                json={"value": value},
            )

    def set_state(self, value: Any):
        self.set_resource(value, path=self.state_path, resource_type="state")

    @property
    def version(self):
        return self.get("version").text

    def get_duckdb_connection_settings(
        self,
        s3_resource_path: str = "",
        none_if_undefined: bool = False,
    ) -> Union[str, None]:
        """
        Convenient helpers that takes an S3 resource as input and returns the settings necessary to
        initiate an S3 connection from DuckDB
        """
        try:
            return self.post(
                f"/w/{self.workspace}/job_helpers/v2/duckdb_connection_settings",
                json={} if s3_resource_path == "" else {"s3_resource_path": s3_resource_path},
            ).json()
        except JSONDecodeError as e:
            if none_if_undefined:
                return None
            raise Exception("Could not generate DuckDB S3 connection settings from the provided resource") from e

    def get_polars_connection_settings(
        self,
        s3_resource_path: str = "",
        none_if_undefined: bool = False,
    ) -> Any:
        """
        Convenient helpers that takes an S3 resource as input and returns the settings necessary to
        initiate an S3 connection from Polars
        """
        try:
            return self.post(
                f"/w/{self.workspace}/job_helpers/v2/polars_connection_settings",
                json={} if s3_resource_path == "" else {"s3_resource_path": s3_resource_path},
            ).json()
        except JSONDecodeError as e:
            if none_if_undefined:
                return None
            raise Exception("Could not generate Polars S3 connection settings from the provided resource") from e

    def whoami(self) -> dict:
        return self.get("/users/whoami").json()

    @property
    def user(self) -> dict:
        return self.whoami()

    @property
    def state_path(self) -> str:
        state_path = os.environ.get("WM_STATE_PATH_NEW", os.environ.get("WM_STATE_PATH"))
        if state_path is None:
            raise Exception("State path not found")
        return state_path

    @property
    def state(self) -> Any:
        return self.get_resource(path=self.state_path, none_if_undefined=True)

    @state.setter
    def state(self, value: Any) -> None:
        self.set_state(value)

    @staticmethod
    def set_shared_state_pickle(value: Any, path: str = "state.pickle") -> None:
        """
        Set the state in the shared folder using pickle
        """
        import pickle

        with open(f"/shared/{path}", "wb") as handle:
            pickle.dump(value, handle, protocol=pickle.HIGHEST_PROTOCOL)

    @staticmethod
    def get_shared_state_pickle(path: str = "state.pickle") -> Any:
        """
        Get the state in the shared folder using pickle
        """
        import pickle

        with open(f"/shared/{path}", "rb") as handle:
            return pickle.load(handle)

    @staticmethod
    def set_shared_state(value: Any, path: str = "state.json") -> None:
        """
        Set the state in the shared folder using pickle
        """
        import json

        with open(f"/shared/{path}", "w", encoding="utf-8") as f:
            json.dump(value, f, ensure_ascii=False, indent=4)

    @staticmethod
    def get_shared_state(path: str = "state.json") -> None:
        """
        Set the state in the shared folder using pickle
        """
        import json

        with open(f"/shared/{path}", "r", encoding="utf-8") as f:
            return json.load(f)

    def get_resume_urls(self, approver: str = None) -> dict:
        nonce = random.randint(0, 1000000000)
        job_id = os.environ.get("WM_JOB_ID") or "NO_ID"
        return self.get(
            f"/w/{self.workspace}/jobs/resume_urls/{job_id}/{nonce}",
            params={"approver": approver},
        ).json()


def init_global_client(f):
    @functools.wraps(f)
    def wrapper(*args, **kwargs):
        global _client
        if _client is None:
            _client = Windmill()
        return f(*args, **kwargs)

    return wrapper


def deprecate(in_favor_of: str):
    def decorator(f):
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            warnings.warn(
                (
                    f"The '{f.__name__}' method is deprecated and may be removed in the future. "
                    f"Consider {in_favor_of}"
                ),
                DeprecationWarning,
            )
            return f(*args, **kwargs)

        return wrapper

    return decorator


@init_global_client
def get_workspace() -> str:
    return _client.workspace


@init_global_client
@deprecate("Windmill().version")
def get_version() -> str:
    return _client.version


@init_global_client
def run_script_async(
    hash_or_path: str,
    args: Dict[str, Any] = None,
    scheduled_in_secs: int = None,
) -> str:
    is_path = "/" in hash_or_path
    hash_ = None if is_path else hash_or_path
    path = hash_or_path if is_path else None
    return _client.run_script_async(
        hash_=hash_,
        path=path,
        args=args,
        scheduled_in_secs=scheduled_in_secs,
    )

@init_global_client
def run_flow_async(
    path: str,
    args: Dict[str, Any] = None,
    scheduled_in_secs: int = None,
) -> str:
    return _client.run_flow_async(
        path=path,
        args=args,
        scheduled_in_secs=scheduled_in_secs,
    )

@init_global_client
def run_script_sync(
    hash: str,
    args: Dict[str, Any] = None,
    verbose: bool = False,
    assert_result_is_not_none: bool = True,
    cleanup: bool = True,
    timeout: dt.timedelta = None,
) -> Any:
    return _client.run_script(
        hash_=hash,
        args=args,
        verbose=verbose,
        assert_result_is_not_none=assert_result_is_not_none,
        cleanup=cleanup,
        timeout=timeout,
    )


@init_global_client
def run_script_by_path_async(
    path: str,
    args: Dict[str, Any] = None,
    scheduled_in_secs: Union[None, int] = None,
) -> str:
    return _client.run_script_async(
        path=path,
        args=args,
        scheduled_in_secs=scheduled_in_secs,
    )


@init_global_client
def run_script_by_path_sync(
    path: str,
    args: Dict[str, Any] = None,
    verbose: bool = False,
    assert_result_is_not_none: bool = True,
    cleanup: bool = True,
    timeout: dt.timedelta = None,
) -> Any:
    return _client.run_script(
        path=path,
        args=args,
        verbose=verbose,
        assert_result_is_not_none=assert_result_is_not_none,
        cleanup=cleanup,
        timeout=timeout,
    )


@init_global_client
def get_job_status(job_id: str) -> JobStatus:
    return _client.get_job_status(job_id)


@init_global_client
def get_result(job_id: str, assert_result_is_not_none=True) -> Dict[str, Any]:
    return _client.get_result(job_id=job_id, assert_result_is_not_none=assert_result_is_not_none)


@init_global_client
def duckdb_connection_settings(s3_resource_path: str = "", none_if_undefined: bool = False) -> Union[str, None]:
    """
    Convenient helpers that takes an S3 resource as input and returns the settings necessary to
    initiate an S3 connection from DuckDB
    """
    return _client.get_duckdb_connection_settings(
        s3_resource_path=s3_resource_path, none_if_undefined=none_if_undefined
    )


@init_global_client
def polars_connection_settings(s3_resource_path: str = "", none_if_undefined: bool = False) -> Any:
    """
    Convenient helpers that takes an S3 resource as input and returns the settings necessary to
    initiate an S3 connection from Polars
    """
    return _client.get_polars_connection_settings(s3_resource_path, none_if_undefined)


@init_global_client
def whoami() -> dict:
    """
    Returns the current user
    """
    return _client.user


@init_global_client
@deprecate("Windmill().state")
def get_state() -> Any:
    """
    Get the state
    """
    return _client.state


@init_global_client
def get_resource(
    path: str,
    none_if_undefined: bool = False,
) -> str | dict | None:
    """Get resource from Windmill"""
    return _client.get_resource(path, none_if_undefined)


@init_global_client
def set_resource(path: str, value: Any, resource_type: str = "any") -> None:
    """
    Set the resource at a given path as a string, creating it if it does not exist
    """
    return _client.set_resource(value=value, path=path, resource_type=resource_type)


@init_global_client
def set_state(value: Any) -> None:
    """
    Set the state
    """
    return _client.set_state(value)


def set_shared_state_pickle(value: Any, path="state.pickle") -> None:
    """
    Set the state in the shared folder using pickle
    """
    return Windmill.set_shared_state_pickle(value=value, path=path)


@deprecate("Windmill.get_shared_state_pickle(...)")
def get_shared_state_pickle(path="state.pickle") -> Any:
    """
    Get the state in the shared folder using pickle
    """
    return Windmill.get_shared_state_pickle(path=path)


def set_shared_state(value: Any, path="state.json") -> None:
    """
    Set the state in the shared folder using pickle
    """
    return Windmill.set_shared_state(value=value, path=path)


def get_shared_state(path="state.json") -> None:
    """
    Set the state in the shared folder using pickle
    """
    return Windmill.get_shared_state(path=path)


@init_global_client
def get_variable(path: str) -> str:
    """
    Returns the variable at a given path as a string
    """
    return _client.get_variable(path)


@init_global_client
def set_variable(path: str, value: str) -> None:
    """
    Set the variable at a given path as a string, creating it if it does not exist
    """
    return _client.set_variable(path, value)


@init_global_client
def get_state_path() -> str:
    return _client.state_path


@init_global_client
def get_resume_urls(approver: str = None) -> dict:
    return _client.get_resume_urls(approver)


@init_global_client
def cancel_running() -> dict:
    """Cancel currently running executions of the same script."""
    return _client.cancel_running()


@init_global_client
def run_script(
    path: str = None,
    hash_: str = None,
    args: dict = None,
    timeout: dt.timedelta | int | float = None,
    verbose: bool = False,
    cleanup: bool = True,
    assert_result_is_not_none: bool = True,
) -> Any:
    """Run script synchronously and return its result."""
    return _client.run_script(
        path=path,
        hash_=hash_,
        args=args,
        verbose=verbose,
        assert_result_is_not_none=assert_result_is_not_none,
        cleanup=cleanup,
        timeout=timeout,
    )
