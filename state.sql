CREATE EXTENSION IF NOT EXISTS debversion;
CREATE DOMAIN distribution_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TABLE IF NOT EXISTS upstream (
   name text,
   upstream_branch_url text,
   primary key(name)
);
CREATE TYPE vcswatch_status AS ENUM('ok', 'error', 'old', 'new', 'commits', 'unrel');
CREATE DOMAIN debian_package_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TYPE vcs_type AS ENUM('bzr', 'git', 'svn', 'mtn', 'hg', 'arch', 'cvs', 'darcs');
CREATE TABLE IF NOT EXISTS codebase (
   branch_url text not null,
   subpath text,
   vcs_last_revision text,
   vcs_type vcs_type,
   unique(branch_url, subpath)
);
CREATE UNIQUE INDEX ON codebase (name);
CREATE INDEX ON codebase (branch_url);
CREATE TABLE IF NOT EXISTS package (
   name debian_package_name not null primary key,
   distribution distribution_name not null,

   -- TODO(jelmer): Move these to codebase
   -- codebase text references codebase(name)
   vcs_type vcs_type,
   branch_url text,
   subpath text,
   vcs_last_revision text,

   maintainer_email text not null,
   uploader_emails text[] not null,
   archive_version debversion,
   vcs_url text,
   vcs_browse text,
   popcon_inst integer,
   removed boolean default false,
   vcswatch_status vcswatch_status,
   vcswatch_version debversion,
   in_base boolean,
   unique(distribution, name)
);
CREATE INDEX ON package (removed);
CREATE INDEX ON package (vcs_url);
CREATE INDEX ON package (branch_url);
CREATE INDEX ON package (maintainer_email);
CREATE INDEX ON package (uploader_emails);
CREATE TYPE merge_proposal_status AS ENUM ('open', 'closed', 'merged', 'applied', 'abandoned');
CREATE TABLE IF NOT EXISTS merge_proposal (
   package text,
   url text not null,
   status merge_proposal_status NULL DEFAULT NULL,
   revision text,
   merged_by text,
   merged_at timestamp,
   foreign key (package) references package(name),
   primary key(url)
);
CREATE INDEX ON merge_proposal (revision);
CREATE INDEX ON merge_proposal (url);
CREATE DOMAIN suite_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TYPE review_status AS ENUM('unreviewed', 'approved', 'rejected');
CREATE TABLE IF NOT EXISTS run (
   id text not null primary key,
   command text,
   description text,
   start_time timestamp,
   finish_time timestamp,
   -- Disabled for now: requires postgresql > 12
   -- duration interval generated always as (finish_time - start_time) stored,
   package text not null,
   result_code text not null,
   instigated_context text,
   -- Some subworker-specific indication of what we attempted to do
   context text,
   -- Main branch revision
   main_branch_revision text,
   branch_name text,
   revision text,
   result json,
   suite suite_name not null,
   branch_url text,
   logfilenames text[] not null,
   review_status review_status not null default 'unreviewed',
   review_comment text,
   value integer,
   -- Name of the worker that executed this run.
   worker text not null,
   -- Link to worker-specific status page
   worker_link text,
   result_tags result_tag[],
   subpath text,
   failure_details json,
   foreign key (package) references package(name)
);
CREATE INDEX ON run (package, suite, start_time DESC);
CREATE INDEX ON run (start_time);
CREATE INDEX ON run (suite, start_time);
CREATE INDEX ON run (package, suite);
CREATE INDEX ON run (suite);
CREATE INDEX ON run (result_code);
CREATE INDEX ON run (revision);
CREATE INDEX ON run (main_branch_revision);
CREATE TYPE publish_mode AS ENUM('push', 'attempt-push', 'propose', 'build-only', 'push-derived', 'skip');
CREATE TABLE IF NOT EXISTS publish (
   id text not null,
   package text not null,
   branch_name text,
   main_branch_revision text,
   revision text,
   role text,
   mode publish_mode not null,
   merge_proposal_url text,
   result_code text not null,
   description text,
   requestor text,
   timestamp timestamp default now(),
   foreign key (package) references package(name),
   foreign key (merge_proposal_url) references merge_proposal(url)
);
CREATE INDEX ON publish (revision);
CREATE INDEX ON publish (merge_proposal_url);
CREATE INDEX ON publish (timestamp);
CREATE TYPE queue_bucket AS ENUM(
    'update-existing-mp', 'webhook', 'manual', 'reschedule', 'control', 'update-new-mp', 'default');
CREATE TABLE IF NOT EXISTS queue (
   id serial,
   bucket queue_bucket default 'default',
   package text not null,
   suite suite_name not null,
   command text not null,
   priority bigint default 0 not null,
   foreign key (package) references package(name),
   -- Some subworker-specific indication of what we are expecting to do.
   context text,
   estimated_duration interval,
   refresh boolean default false,
   requestor text,
   unique(package, suite)
);
CREATE INDEX ON queue (priority ASC, id ASC);
CREATE INDEX ON queue (bucket ASC, priority ASC, id ASC);
CREATE TABLE IF NOT EXISTS candidate (
   package text not null,
   suite suite_name not null,
   context text,
   value integer,
   success_chance float,
   unique(package, suite),
   foreign key (package) references package(name)
);
CREATE TYPE changelog_mode AS ENUM('auto', 'update', 'leave');
CREATE TABLE IF NOT EXISTS publish_policy (
   role text not null,
   mode publish_mode default 'build-only',
   frequency_days int,
   unique(role)
);
CREATE TABLE IF NOT EXISTS policy (
   package text not null,
   suite suite_name not null,
   update_changelog changelog_mode default 'auto',
   publish publish_policy[],
   command text,
   foreign key (package) references package(name),
   unique(package, suite)
);
CREATE INDEX ON candidate (suite);
CREATE TABLE worker (
   name text not null unique,
   password text not null,
   link text
);

-- The last run per package/suite
CREATE VIEW last_runs AS
  SELECT DISTINCT ON (package, suite)
  *
  FROM
  run
  WHERE NOT EXISTS (SELECT FROM package WHERE name = package and removed)
  ORDER BY package, suite, start_time DESC;

-- The last effective run per package/suite; i.e. the last run that
-- wasn't an attempt to incrementally improve things that yielded no new
-- changes.
CREATE OR REPLACE VIEW last_effective_runs AS
  SELECT DISTINCT ON (package, suite)
  *
  FROM
  run
  WHERE
    NOT EXISTS (SELECT FROM package WHERE name = package and removed) AND
    result_code != 'nothing-new-to-do'
  ORDER BY package, suite, start_time DESC;

CREATE OR REPLACE VIEW absorbed_revisions AS
   SELECT revision FROM publish WHERE revision IS NOT NULL AND ((mode = 'push' and result_code = 'success') OR (mode = 'propose' AND result_code = 'empty-merge-proposal'))
 UNION
   SELECT revision FROM merge_proposal WHERE revision IS NOT NULL AND status in ('merged', 'applied');

-- The last "unabsorbed" change. An unabsorbed change is the last change that
-- was not yet merged or pushed.
CREATE OR REPLACE VIEW last_unabsorbed_runs AS
  SELECT * FROM last_effective_runs WHERE
     -- Either the last run is unabsorbed because it failed:
     result_code NOT in ('nothing-to-do', 'success')
     -- or because one of the result branch revisions has not been absorbed yet
     OR id in (SELECT run_id from new_result_branch WHERE revision NOT IN (SELECT * FROM absorbed_revisions));

create or replace view suites as select distinct suite as name from run;

CREATE OR REPLACE VIEW absorbed_runs AS
  SELECT * FROM run WHERE result_code = 'success' and
  exists (select from new_result_branch WHERE run_id = run.id) and
  not exists (select from new_result_branch WHERE run_id = run.id AND revision not in (SELECT revision FROM absorbed_revisions));

CREATE OR REPLACE VIEW absorbed_lintian_fixes AS
  select absorbed_runs.*, x.summary, x.description as fix_description, x.certainty, x.fixed_lintian_tags from absorbed_runs, json_to_recordset((result->'applied')::json) as x("summary" text, "description" text, "certainty" text, "fixed_lintian_tags" text[]);

CREATE OR REPLACE VIEW last_unabsorbed_lintian_fixes AS
  select last_unabsorbed_runs.*, x.summary, x.description as fix_description, x.certainty, x.fixed_lintian_tags from last_unabsorbed_runs, json_to_recordset((result->'applied')::json) as x("summary" text, "description" text, "certainty" text, "fixed_lintian_tags" text[]) WHERE result_code = 'success';

CREATE OR REPLACE VIEW perpetual_candidates AS
  select suite, package from candidate union select suite, package from run;

CREATE OR REPLACE VIEW first_run_time AS
 SELECT DISTINCT ON (run.package, run.suite) run.package, run.suite, run.start_time
 FROM run ORDER BY run.package, run.suite;

CREATE OR REPLACE FUNCTION drop_candidates_for_deleted_packages()
  RETURNS TRIGGER
  LANGUAGE PLPGSQL
  AS
$$
BEGIN
    IF NEW.removed AND NOT OLD.removed THEN
        DELETE FROM candidate WHERE package = NEW.name;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER drop_candidates_when_removed
  AFTER UPDATE OF removed
  ON package
  FOR EACH ROW
  EXECUTE PROCEDURE drop_candidates_for_deleted_packages();

CREATE OR REPLACE VIEW absorbed_multiarch_hints AS
  select package, id, x->>'binary' as binary, x->>'link'::text as link, x->>'severity' as severity, x->>'source' as source, (x->>'version')::debversion as version, x->'action' as action, x->>'certainty' as certainty from (select package, id, json_array_elements(result->'applied-hints') as x from absorbed_runs where suite = 'multiarch-fixes') as f;

CREATE OR REPLACE VIEW multiarch_hints AS
  select package, id, x->>'binary' as binary, x->>'link'::text as link, x->>'severity' as severity, x->>'source' as source, (x->>'version')::debversion as version, x->'action' as action, x->>'certainty' as certainty from (select package, id, json_array_elements(result->'applied-hints') as x from run where suite = 'multiarch-fixes') as f;

CREATE TABLE site_session (
  id text primary key,
  timestamp timestamp not null default now(),
  userinfo json
);


CREATE FUNCTION expire_site_session_delete_old_rows() RETURNS trigger
  LANGUAGE PLPGSQL
  AS
$$
BEGIN
  DELETE FROM site_session WHERE timestamp < NOW() - INTERVAL '1 week';
  RETURN NEW;
END;
$$;

CREATE TRIGGER expire_site_session_delete_old_rows_trigger
   AFTER INSERT ON site_session
   EXECUTE PROCEDURE expire_site_session_delete_old_rows();

CREATE VIEW queue_positions AS SELECT
    package,
    suite,
    row_number() OVER (ORDER BY priority ASC, id ASC) AS position,
    SUM(estimated_duration) OVER (ORDER BY priority ASC, id ASC)
        - coalesce(estimated_duration, interval '0') AS wait_time
FROM
    queue
ORDER BY bucket ASC, priority ASC, id ASC;

CREATE TABLE debian_build (
 run_id text not null references run (id),
 -- Debian version text of the built package
 version debversion not null,
 -- Distribution the package was built for (e.g. "lintian-fixes")
 distribution text not null,
 source text not null,
 lintian_result json
);
CREATE INDEX ON debian_build (run_id);
CREATE INDEX ON debian_build (distribution, source, version);

CREATE TABLE result_branch (
 role text not null,
 remote_name text not null,
 base_revision text not null,
 revision text not null
);

CREATE TABLE new_result_branch (
 run_id text not null references run (id),
 role text not null,
 remote_name text,
 base_revision text not null,
 revision text not null,
 UNIQUE(run_id, role)
);

CREATE INDEX ON new_result_branch (revision);

CREATE TABLE result_tag (
 actual_name text,
 revision text not null
);

CREATE INDEX ON result_tag (revision);

CREATE TYPE result_branch_with_policy AS (
  role text,
  remote_name text,
  base_revision text,
  revision text,
  mode publish_mode,
  frequency_days integer);

CREATE OR REPLACE VIEW publishable AS
  SELECT
  run.id AS id,
  run.command AS command,
  run.start_time AS start_time,
  run.finish_time AS finish_time,
  run.description AS description,
  run.package AS package,
  debian_build.version AS build_version,
  debian_build.distribution AS build_distribution,
  run.result_code AS result_code,
  run.branch_name AS branch_name,
  run.main_branch_revision AS main_branch_revision,
  run.revision AS revision,
  run.context AS context,
  run.result AS result,
  run.suite AS suite,
  run.instigated_context AS instigated_context,
  run.branch_url AS branch_url,
  run.logfilenames AS logfilenames,
  run.review_status AS review_status,
  run.review_comment AS review_comment,
  run.worker AS worker,
  array(SELECT ROW(role, remote_name, base_revision, revision)::result_branch FROM new_result_branch WHERE new_result_branch.run_id = run.id) as result_branches,
  run.result_tags AS result_tags,
  run.value AS value,
  package.maintainer_email AS maintainer_email,
  package.uploader_emails AS uploader_emails,
  policy.update_changelog AS update_changelog,
  policy.command AS policy_command,
  ARRAY(
   SELECT row(rb.role, remote_name, base_revision, revision, mode, frequency_days)::result_branch_with_policy
   FROM new_result_branch rb
    LEFT JOIN UNNEST(policy.publish) pp ON pp.role = rb.role
   WHERE rb.run_id = run.id AND revision NOT IN (SELECT revision FROM absorbed_revisions)
  ) AS unpublished_branches
FROM
  last_effective_runs AS run
INNER JOIN package ON package.name = run.package
INNER JOIN policy ON
    policy.package = run.package AND policy.suite = run.suite
LEFT JOIN debian_build ON run.id = debian_build.run_id
WHERE
  result_code = 'success' AND NOT package.removed;

CREATE OR REPLACE VIEW publish_ready AS SELECT * FROM publishable WHERE ARRAY_LENGTH(unpublished_branches, 1) > 0;

CREATE VIEW upstream_branch_urls as (
    select package, result->>'upstream_branch_url' as url from run where suite in ('fresh-snapshots', 'fresh-releases') and result->>'upstream_branch_url' != '')
union
    (select name as package, upstream_branch_url as url from upstream);

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
    branch_name,
    main_branch_revision,
    revision,
    context,
    result,
    suite,
    instigated_context,
    branch_url,
    logfilenames,
    review_status,
    review_comment,
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
