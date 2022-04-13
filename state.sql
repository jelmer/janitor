CREATE EXTENSION IF NOT EXISTS debversion;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE TABLE IF NOT EXISTS upstream (
   name text,
   upstream_branch_url text,
   primary key(name)
);
CREATE TYPE vcs_type AS ENUM('bzr', 'git', 'svn', 'mtn', 'hg', 'arch', 'cvs', 'darcs');
CREATE DOMAIN codebase_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TABLE IF NOT EXISTS codebase (
   name codebase_name,
   branch_url text not null,
   subpath text,
   vcs_last_revision text,
   vcs_type vcs_type,
   unique(branch_url, subpath),
   unique(name)
);
CREATE INDEX ON codebase (branch_url);
CREATE INDEX ON codebase (name);

CREATE TYPE vcswatch_status AS ENUM('ok', 'error', 'old', 'new', 'commits', 'unrel');
CREATE DOMAIN distribution_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE DOMAIN debian_package_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TABLE IF NOT EXISTS package (
   name debian_package_name not null primary key,
   distribution distribution_name not null,

   -- TODO(jelmer): Move these to codebase
   codebase text references codebase(name),
   vcs_type vcs_type,
   branch_url text,
   subpath text,
   vcs_last_revision text,

   maintainer_email text,
   uploader_emails text[],
   archive_version debversion,
   vcs_url text,
   vcs_browse text,
   popcon_inst integer,
   removed boolean default false,
   vcswatch_status vcswatch_status,
   vcswatch_version debversion,
   in_base boolean,
   origin text,
   unique(distribution, name)
);
CREATE INDEX ON package (removed);
CREATE INDEX ON package (vcs_url);
CREATE INDEX ON package (branch_url);
CREATE INDEX ON package (maintainer_email);
CREATE INDEX ON package (uploader_emails);
CREATE TYPE merge_proposal_status AS ENUM ('open', 'closed', 'merged', 'applied', 'abandoned', 'rejected');
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
CREATE DOMAIN campaign_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TYPE review_status AS ENUM('unreviewed', 'approved', 'rejected', 'abstained');
CREATE TABLE result_tag (
 actual_name text,
 revision text not null
);

CREATE INDEX ON result_tag (revision);
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
   revision text,
   result json,
   suite suite_name not null, -- DEPRECATED
   vcs_type vcs_type,
   branch_url text,
   logfilenames text[] not null,
   review_status review_status not null default 'unreviewed',
   review_comment text,
   value integer,
   -- Name of the worker that executed this run.
   worker text references worker(name),
   worker_link text,
   result_tags result_tag[],
   subpath text,
   failure_details json,
   target_branch_url text,
   resume_from text references run (id),
   change_set text references change_set(id),
   foreign key (package) references package(name),
   check(finish_time >= start_time)
);
CREATE INDEX ON run (package, suite, start_time DESC);
CREATE INDEX ON run (start_time);
CREATE INDEX ON run (suite, start_time);
CREATE INDEX ON run (package, suite);
CREATE INDEX ON run (suite);
CREATE INDEX ON run (result_code);
CREATE INDEX ON run (revision);
CREATE INDEX ON run (main_branch_revision);
CREATE INDEX ON run (change_set);
CREATE TYPE publish_mode AS ENUM('push', 'attempt-push', 'propose', 'build-only', 'push-derived', 'skip', 'bts');
CREATE TYPE review_policy AS ENUM('not-required', 'required');
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
    'update-existing-mp', 'manual', 'control', 'hook', 'reschedule', 'update-new-mp', 'missing-deps', 'default');
CREATE TABLE IF NOT EXISTS queue (
   id serial,
   bucket queue_bucket default 'default',
   package text not null,
   suite suite_name not null,
   command text,
   priority bigint default 0 not null,
   foreign key (package) references package(name),
   -- Some subworker-specific indication of what we are expecting to do.
   context text,
   estimated_duration interval,
   refresh boolean default false,
   requestor text,
   change_set text references change_set(id),
   unique(package, suite, change_set)
);
CREATE INDEX ON queue (change_set);
CREATE INDEX ON queue (priority ASC, id ASC);
CREATE INDEX ON queue (bucket ASC, priority ASC, id ASC);
CREATE TABLE IF NOT EXISTS candidate (
   package text not null,
   suite suite_name not null,
   context text,
   value integer,
   success_chance float,
   change_set text references change_set(id),
   unique(package, suite, change_set),
   foreign key (package) references package(name)
);
CREATE TABLE IF NOT EXISTS branch_publish_policy (
   role text not null,
   mode publish_mode default 'build-only',
   frequency_days int,
   unique(role)
);
CREATE TABLE IF NOT EXISTS publish_policy (
   package text not null,
   campaign campaign_name not null,
   per_branch_policy branch_publish_policy[]
   qa_review review_policy,
   foreign key (package) references package(name),
   unique(package, campaign)
);
CREATE TYPE notify_mode AS ENUM('no_notification', 'email', 'bts');
CREATE TABLE IF NOT EXISTS policy (
   package text not null,
   suite suite_name not null,
   command text,
   broken_notify notify_mode,
   foreign key (package) references package(name),
   unique(package, suite)
);
CREATE INDEX ON candidate (suite);
CREATE INDEX ON candidate(change_set);
CREATE TABLE IF NOT EXISTS worker (
   name text not null unique,
   password text not null,
   link text
);

-- The last run per package/suite
CREATE OR REPLACE VIEW last_runs AS
  SELECT DISTINCT ON (package, suite)
  *
  FROM
  run
  WHERE NOT EXISTS (SELECT FROM package WHERE name = package and removed)
  ORDER BY package, suite, change_set, start_time DESC;

-- The last effective run per package/suite; i.e. the last run that
-- wasn't an attempt to incrementally improve things that yielded no new
-- changes.
CREATE VIEW last_effective_runs AS
  SELECT DISTINCT ON (package, suite)
  *
  FROM
  run
  WHERE
    result_code != 'nothing-new-to-do'
  ORDER BY package, suite, change_set, start_time DESC;

CREATE TABLE new_result_branch (
 run_id text not null references run (id),
 role text not null,
 remote_name text,
 base_revision text,
 revision text not null,
 absorbed boolean default false,
 UNIQUE(run_id, role)
);

CREATE INDEX ON new_result_branch (revision);
CREATE INDEX ON new_result_branch (absorbed);

-- The last "unabsorbed" change. An unabsorbed change is the last change that
-- was not yet merged or pushed.
CREATE VIEW last_unabsorbed_runs AS
  SELECT last_effective_runs.* FROM last_effective_runs INNER JOIN package ON package.name = last_effective_runs.package WHERE
     -- Either the last run is unabsorbed because it failed:
     (result_code NOT in ('nothing-to-do', 'success')
     -- or because one of the result branch revisions has not been absorbed yet
      OR exists (SELECT from new_result_branch WHERE run_id = id and not absorbed)) AND NOT package.removed;

CREATE OR REPLACE FUNCTION notify_run_update()
  RETURNS TRIGGER AS $$
   DECLARE
    row RECORD;

    BEGIN
    -- Checking the Operation Type
    IF (TG_OP = 'DELETE') THEN
      row = OLD;
    ELSE
      row = NEW;
    END IF;

    -- Calling the pg_notify for my_table_update event with output as payload
    PERFORM pg_notify('run_update', row.id);

    -- Returning null because it is an after trigger.
    RETURN NULL;
    END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER notify_run_updates
  AFTER INSERT OR UPDATE OR DELETE
  ON run
  FOR EACH ROW
  EXECUTE PROCEDURE notify_run_update();

create or replace view suites as select distinct suite as name from run;

CREATE OR REPLACE VIEW absorbed_runs AS
  SELECT * FROM run WHERE result_code = 'success' and
  exists (select from new_result_branch WHERE run_id = run.id) and
  not exists (select from new_result_branch WHERE run_id = run.id AND not absorbed);

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

CREATE OR REPLACE VIEW queue_positions AS SELECT
    package,
    suite,
    row_number() OVER (ORDER BY bucket ASC, priority ASC, id ASC) AS position,
    SUM(estimated_duration) OVER (ORDER BY priority ASC, id ASC)
        - coalesce(estimated_duration, interval '0') AS wait_time
FROM
    queue
ORDER BY bucket ASC, priority ASC, id ASC;

CREATE TABLE result_branch (
 role text not null,
 remote_name text not null,
 base_revision text not null,
 revision text not null
);

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
  run.result_code AS result_code,
  run.main_branch_revision AS main_branch_revision,
  run.revision AS revision,
  run.context AS context,
  run.result AS result,
  run.suite AS suite,
  run.instigated_context AS instigated_context,
  run.vcs_type AS vcs_type,
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
  policy.command AS policy_command,
  policy.qa_review AS qa_review_policy,
  (policy.qa_review = 'required' AND review_status = 'unreviewed') as needs_review,
  ARRAY(
   SELECT row(rb.role, remote_name, base_revision, revision, mode, frequency_days)::result_branch_with_policy
   FROM new_result_branch rb
    LEFT JOIN UNNEST(policy.publish) pp ON pp.role = rb.role
   WHERE rb.run_id = run.id AND not absorbed
   ORDER BY rb.role != 'main' DESC
  ) AS unpublished_branches,
  target_branch_url,
  run.change_set AS change_set
FROM
  last_effective_runs AS run
INNER JOIN package ON package.name = run.package
INNER JOIN policy ON
    policy.package = run.package AND policy.suite = run.suite
WHERE
  result_code = 'success' AND NOT package.removed;

CREATE OR REPLACE VIEW publish_ready AS SELECT * FROM publishable WHERE ARRAY_LENGTH(unpublished_branches, 1) > 0;

CREATE VIEW upstream_branch_urls as (
    select package, result->>'upstream_branch_url' as url from run where suite in ('fresh-snapshots', 'fresh-releases') and result->>'upstream_branch_url' != '')
union
    (select name as package, upstream_branch_url as url from upstream);

CREATE TABLE IF NOT EXISTS review (
 run_id text not null references run (id),
 comment text,
 reviewer text,
 review_status review_status not null default 'unreviewed',
 reviewed_at timestamp not null default now()
);
CREATE INDEX ON review (run_id);
CREATE UNIQUE INDEX ON review (run_id, reviewer);

CREATE TYPE change_set_state AS ENUM ('working', 'ready', 'publishing', 'done');

CREATE TABLE IF NOT EXISTS change_set (
  id text not null primary key,
  initial_run_id text references run(id)
  campaign campaign_name not null,
  state change_set_state default 'working' not null
);
