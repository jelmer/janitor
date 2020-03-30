CREATE TYPE vcswatch_status AS ENUM('ok', 'error', 'old', 'new', 'commits', 'unrel');
CREATE DOMAIN package_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TABLE IF NOT EXISTS package (
   name package_name,
   branch_url text not null,
   subpath text,
   maintainer_email text not null,
   uploader_emails text[] not null,
   unstable_version debversion,
   vcs_type text,
   vcs_url text,
   vcs_browse text,
   popcon_inst integer,
   removed boolean default false,
   vcswatch_status vcswatch_status,
   vcswatch_version debversion,
   upstream_branch_url text,
   primary key(name)
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
CREATE DOMAIN suite_name AS TEXT check (value similar to '[a-z0-9][a-z0-9+-.]+');
CREATE TYPE review_status AS ENUM('unreviewed', 'approved', 'rejected');
CREATE TABLE IF NOT EXISTS run (
   id text not null primary key,
   command text,
   description text,
   start_time timestamp,
   finish_time timestamp,
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
   result jsonb,
   suite suite_name not null,
   branch_url text not null,
   logfilenames text[] not null,
   review_status review_status not null default 'unreviewed',
   value integer,
   foreign key (package) references package(name),
);
CREATE INDEX ON run (package, suite, start_time DESC);
CREATE INDEX ON run (start_time);
CREATE INDEX ON run (suite, start_time);
CREATE INDEX ON run (package, suite);
CREATE INDEX ON run (suite);
CREATE INDEX ON run (build_distribution);
CREATE INDEX ON run (result_code);
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
CREATE TABLE IF NOT EXISTS queue (
   id serial,
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
CREATE TABLE IF NOT EXISTS publish_policy (
   package text not null,
   suite suite_name not null,
   mode publish_mode default 'build-only',
   update_changelog changelog_mode default 'auto',
   command text[],
   foreign key (package) references package(name),
   unique(package, suite)
);
CREATE INDEX ON candidate (suite);
CREATE OR REPLACE VIEW last_runs AS
  SELECT DISTINCT ON (package, suite)
  *
  FROM
  run
  WHERE NOT EXISTS (SELECT FROM package WHERE name = package and removed)
  ORDER BY package, suite, start_time DESC;

CREATE OR REPLACE VIEW last_effective_runs AS
  SELECT DISTINCT ON (package, suite)
  *
  FROM
  run
  WHERE
    NOT EXISTS (SELECT FROM package WHERE name = package and removed) AND
    result_code != 'nothing-new-to-do'
  ORDER BY package, suite, start_time DESC;

CREATE OR REPLACE VIEW last_unabsorbed_runs AS
  SELECT * FROM last_effective_runs WHERE
     result_code NOT in ('nothing-to-do', 'success') OR (
     revision is not null AND
     revision != main_branch_revision AND
     revision NOT IN (SELECT revision FROM publish WHERE (mode = 'push' and result_code = 'success') OR (mode = 'propose' AND result_code = 'empty-merge-proposal')) AND
     revision NOT IN (SELECT revision FROM merge_proposal WHERE status in ('merged', 'applied')));

CREATE OR REPLACE VIEW merged_runs AS
  SELECT run.*, merge_proposal.url, merge_proposal.merged_by
  FROM run
  INNER JOIN merge_proposal ON merge_proposal.revision = run.revision
  WHERE result_code = 'success' and merge_proposal.status in ('merged', 'applied');

create or replace view suites as select distinct suite as name from run;

CREATE OR REPLACE VIEW absorbed_runs AS
  SELECT * FROM run WHERE result_code = 'success' and revision in (select revision from publish where mode = 'push' and result_code = 'success') or revision in (select revision from merge_proposal where status in ('merged', 'applied'));

CREATE OR REPLACE VIEW absorbed_lintian_fixes AS
  select absorbed_runs.*, x.summary, x.description as fix_description, x.certainty, x.fixed_lintian_tags from absorbed_runs, json_to_recordset(result->'applied') as x("summary" text, "description" text, "certainty" text, "fixed_lintian_tags" text[]);


CREATE OR REPLACE VIEW last_unabsorbed_lintian_fixes AS
  select last_unabsorbed_runs.*, x.summary, x.description as fix_description, x.certainty, x.fixed_lintian_tags from last_unabsorbed_runs, json_to_recordset(result->'applied') as x("summary" text, "description" text, "certainty" text, "fixed_lintian_tags" text[]) WHERE result_code = 'success';
