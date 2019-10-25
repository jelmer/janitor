select last_runs.package, last_runs.build_version, run.build_version from last_runs join run on last_runs.main_branch_revision = run.main_branch_revision where run.build_version is not null and run.suite = 'unchanged' and last_runs.suite = 'lintian-fixes' and last_runs.build_version is not null and run.result_code = 'success';


$ dget https://janitor.debian.net/{suite}/{package}_{version}_amd64.changes
$ dget https://janitor.debian.net/unchanged/{package}_{version}_amd64.changes
$ debdiff unchanged/{package}_{version}_amd64.changes {package}_{version}_amd64.changes
