from breezy.branch import Branch

def is_alioth_url(url: str) -> bool:
    ...

def is_authenticated_url(url: str) -> bool:
    ...

def get_branch_vcs_type(branch: Branch) -> str:
    ...
