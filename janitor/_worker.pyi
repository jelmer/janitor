from typing import Any, Optional, TypedDict
from datetime import datetime

async def is_gce_instance() -> bool: ...
async def gce_external_ip() -> str: ...

class EmptyQueue(Exception): ...
class AssignmentFailure(Exception): ...
class ResultUploadFailure(Exception): ...
class LintianOutputInvalid(Exception): ...

async def abort_run(client: Client, run_id: str, metadata: Any, description: str) -> None: ...

class Client(object):
    def __new__(cls, base_url: str, username: str | None, password: str | None, user_agent: str) -> Client: ...

    async def get_assignment_raw(self, my_url: str | None, node_name: str,
                                jenkins_build_url: str | None, codebase: str | None,
                                 campaign: str | None) -> Any: ...

    async def upload_results(self, run_id: str, metadata: Any, output_directory: str | None = None) -> Any: ...


def run_lintian(output_directory: str, changes_names: list[str], profile: str | None, suppress_tags: list[str] | None) -> Any: ...


class WorkerFailure(Exception):

    def __new__(cls, code: str, description: str, details: Any | None = None, stage: tuple[str, ...] | None = None, transient: bool | None = None) -> WorkerFailure: ...

    code: str
    description: str
    details: Any | None
    stage: tuple[str, ...] | None
    transient: bool | None


class MetadataTarget:
    name: str
    details: Any | None


class Metadata:
    code: str | None
    codebase: str | None
    command: list[str] | None
    description: str | None
    revision: bytes | None
    main_branch_revision: bytes | None
    subpath: str | None
    target_branch_url: str | None
    refreshed: bool | None
    codemod: Any | None
    branch_url: Optional[str] | None
    vcs_type: str | None
    branches: list[tuple[str, Optional[str], Optional[bytes], Optional[bytes]]] | None
    tags: list[tuple[str, bytes]] | None
    value: int | None
    target_name: str | None
    target_details: Any | None
    remotes: dict[str, dict[str, str]] | None
    start_time: datetime | None
    finish_time: datetime | None
    queue_id: int | None

    def add_tag(self, tag: str, value: bytes) -> None: ...
    def add_branch(self, function: str, name: str | None, base_revision: bytes | None, revision: bytes | None) -> None: ...
    def add_remote(self, name: str, url: str) -> None: ...
    def update(self, e: WorkerFailure) -> None: ...
    def json(self) -> Any: ...
