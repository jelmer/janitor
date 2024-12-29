from breezy.branch import Branch
from typing import List, Optional

class ArtifactManager:
    async def store_artifacts(self, run_id: str, local_path: str, names: Optional[List[str]] = None) -> None: ...

def is_alioth_url(url: str) -> bool: ...
def is_authenticated_url(url: str) -> bool: ...
def get_branch_vcs_type(branch: Branch) -> str: ...
async def store_artifacts_with_backup(
    manager: ArtifactManager,
    backup_manager: Optional[ArtifactManager],
    from_dir: str,
    run_id: str,
    names: Optional[List[str]] = None,
) -> None: ...
async def upload_backup_artifacts(
    backup_manager: ArtifactManager,
    manager: ArtifactManager,
    timeout: Optional[int] = None,
) -> None: ...
