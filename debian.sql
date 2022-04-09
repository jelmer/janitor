CREATE TABLE debian_build (
 run_id text not null references run (id),
 -- Debian version text of the built package
 version debversion not null,
 -- Distribution the package was built for (e.g. "lintian-fixes")
 distribution text not null,
 source text not null,
 binary_packages text[],
 lintian_result json
);
CREATE INDEX ON debian_build (run_id);
CREATE INDEX ON debian_build (distribution, source, version);


CREATE OR REPLACE VIEW debian_run AS
SELECT
    id,
    command,
    start_time,
    finish_time,
    description,
    package,
    debian_build.version AS build_version,
    debian_build.distribution AS build_distribution,
    debian_build.lintian_result AS lintian_result,
    result_code,
    main_branch_revision,
    revision,
    context,
    result,
    suite,
    instigated_context,
    branch_url,
    logfilenames,
    worker,
    result_tags
FROM
    run
LEFT JOIN
    debian_build ON debian_build.run_id = run.id;


CREATE VIEW all_debian_versions AS
SELECT
  source,
  distribution,
  version
FROM
  debian_build

UNION

SELECT
  name AS source,
  distribution,
  archive_version AS version
FROM
  package;

create view last_missing_apt_dependencies as select id, package, suite, relation.* from last_unabsorbed_runs, json_array_elements(failure_details->'relations') as relations, json_to_recordset(relations) as relation(name text, archqual text, version text[], arch text, restrictions text) where result_code = 'install-deps-unsatisfied-apt-dependencies';


