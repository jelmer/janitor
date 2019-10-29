CREATE TABLE IF NOT EXISTS package (
   name text not null,
   branch_url text not null,
   maintainer_email text not null,
   uploader_emails text[] not null,
   unstable_version debversion,
   vcs_type text,
   vcs_url text,
   vcs_browse text,
   popcon_inst integer,
   removed boolean default false,
   primary key(name)
);
CREATE INDEX ON package (removed);
CREATE INDEX ON package (vcs_url);
CREATE INDEX ON package (branch_url);
CREATE INDEX ON package (maintainer_email);
CREATE INDEX ON package (uploader_emails);
CREATE TYPE merge_proposal_status AS ENUM ('open', 'closed', 'merged');
CREATE TABLE IF NOT EXISTS merge_proposal (
   package text,
   url text not null,
   status merge_proposal_status NULL DEFAULT NULL,
   revision text,
   foreign key (package) references package(name),
   primary key(url)
);
CREATE TYPE suite AS ENUM('lintian-fixes', 'fresh-releases', 'fresh-snapshots','unchanged');
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
   suite suite not null,
   branch_url text not null,
   logfilenames text[] not null,
   foreign key (package) references package(name),
   foreign key (merge_proposal_url) references merge_proposal(url)
);
CREATE INDEX ON run (package, suite, start_time DESC);
CREATE INDEX ON run (start_time);
CREATE INDEX ON run (suite, start_time);
CREATE INDEX ON run (package, suite);
CREATE INDEX ON run (suite);
CREATE INDEX ON run (build_distribution);
CREATE INDEX ON run (result_code);
CREATE TYPE publish_mode AS ENUM('push', 'attempt-push', 'propose', 'build-only', 'push-derived');
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
   timestamp timestamp default now(),
   foreign key (package) references package(name),
   foreign key (merge_proposal_url) references merge_proposal(url)
);
CREATE TABLE IF NOT EXISTS queue (
   id serial,
   branch_url text not null,
   package text not null,
   suite suite not null,
   command text not null,
   committer text null,
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
   revision text,
   last_scanned timestamp,
   status text,
   description text
);
CREATE TABLE IF NOT EXISTS candidate (
   package text not null,
   suite suite not null,
   context text,
   value integer,
   unique(package, suite),
   foreign key (package) references package(name)
);
CREATE INDEX ON candidate (suite);
CREATE OR REPLACE VIEW last_runs AS
  SELECT DISTINCT ON (package, suite)
  *
  FROM
  run
  WHERE NOT EXISTS (SELECT FROM package WHERE name = package and removed)
  ORDER BY package, suite, start_time DESC;

CREATE OR REPLACE VIEW unabsorbed_runs AS
  SELECT * FROM last_runs WHERE
     result_code NOT in ('nothing-to-do', 'success') OR (
     revision is not null AND
     revision != main_branch_revision AND
     revision NOT IN (SELECT revision FROM publish WHERE (mode = 'push' and result_code = 'success') OR (mode = 'propose' AND result_code = 'empty-merge-proposal')) AND
     revision NOT IN (SELECT revision FROM merge_proposal WHERE status = 'merged'));
