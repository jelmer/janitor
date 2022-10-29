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


