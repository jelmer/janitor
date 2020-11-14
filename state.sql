CREATE EXTENSION IF NOT EXISTS debversion;
CREATE DOMAIN distribution_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TABLE IF NOT EXISTS upstream (
   name text,
   upstream_branch_url text,
   primary key(name)
);
CREATE TYPE vcswatch_status AS ENUM('ok', 'error', 'old', 'new', 'commits', 'unrel');
CREATE DOMAIN package_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TABLE IF NOT EXISTS package (
   name package_name not null primary key,
   distribution distribution_name not null,
   branch_url text,
   subpath text,
   maintainer_email text not null,
   uploader_emails text[] not null,
   archive_version debversion,
   vcs_type text,
   vcs_url text,
   vcs_browse text,
   vcs_last_revision text,
   popcon_inst integer,
   removed boolean default false,
   vcswatch_status vcswatch_status,
   vcswatch_version debversion,
   unique(distribution, name)
);
CREATE INDEX ON package (removed);
CREATE INDEX ON package (vcs_url);
CREATE INDEX ON package (branch_url);
CREATE INDEX ON package (maintainer_email);
CREATE INDEX ON package (uploader_emails);
CREATE TYPE merge_proposal_status AS ENUM ('open', 'closed', 'merged', 'applied');
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
   duration interval generated always as (finish_time - start_time) stored,
   package text not null,
   -- Debian version text of the built package
   build_version debversion,
   -- Distribution the package was built for (e.g. "lintian-fixes")
   build_distribution text,
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
   foreign key (package) references package(name)
);
CREATE INDEX ON run (package, suite, start_time DESC);
CREATE INDEX ON run (start_time);
CREATE INDEX ON run (suite, start_time);
CREATE INDEX ON run (package, suite);
CREATE INDEX ON run (suite);
CREATE INDEX ON run (build_distribution);
CREATE INDEX ON run (result_code);
CREATE INDEX ON run (revision);
CREATE INDEX ON run (main_branch_revision);
CREATE INDEX ON run (duration);
CREATE TYPE publish_mode AS ENUM('push', 'attempt-push', 'propose', 'build-only', 'push-derived', 'skip');
CREATE TABLE IF NOT EXISTS publish (
   id text not null,
   package text not null,
   branch_name text,
   main_branch_revision text,
   revision text,
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
CREATE TABLE IF NOT EXISTS branch (
   url text not null primary key,
   canonical_url text,
   revision text,
   last_scanned timestamp,
   status text,
   description text
);
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
CREATE TABLE IF NOT EXISTS policy (
   package text not null,
   suite suite_name not null,
   update_changelog changelog_mode default 'auto',
   command text,
   foreign key (package) references package(name),
   unique(package, suite)
);
CREATE TABLE IF NOT EXISTS publish_policy (
   package text not null,
   suite suite_name not null,
   role text not null,
   mode publish_mode default 'build-only',
   foreign key (package) references package(name),
   unique(package, suite, text)
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
     result_code NOT in ('nothing-to-do', 'success') OR (
     revision is not null AND
     revision != main_branch_revision AND
     revision NOT IN (SELECT revision FROM absorbed_revisions));

CREATE OR REPLACE VIEW merged_runs AS
  SELECT run.*, merge_proposal.url, merge_proposal.merged_by
  FROM run
  INNER JOIN merge_proposal ON merge_proposal.revision = run.revision
  WHERE result_code = 'success' and merge_proposal.status in ('merged', 'applied');

create or replace view suites as select distinct suite as name from run;

CREATE OR REPLACE VIEW absorbed_runs AS
  SELECT * FROM run WHERE result_code = 'success' and revision in (SELECT revision FROM absorbed_revisions);

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
 build_version debversion not null,
 -- Distribution the package was built for (e.g. "lintian-fixes")
 build_distribution text not null,
 source text not null
);

CREATE TABLE result_branch (
 run_id text not null references run (id),
 role text,
 remote_name text not null,
 base_revision text,
 revision text
);

CREATE UNIQUE INDEX ON result_branch (run_id, remote_name);
CREATE INDEX ON result_branch (revision);

CREATE TABLE result_tag (
 run_id text not null references run (id),
 actual_name text,
 revision text
);

CREATE UNIQUE INDEX ON result_tag (run_id, actual_name);
CREATE INDEX ON result_tag (revision);
