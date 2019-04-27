CREATE TABLE IF NOT EXISTS package (
   name text not null,
   branch_url text not null,
   maintainer_email text not null,
   primary key(name),
);
CREATE TYPE merge_proposal_status AS ENUM ('open', 'closed', 'merged');
CREATE TABLE IF NOT EXISTS merge_proposal (
   package text,
   url text not null,
   status merge_proposal_status NULL DEFAULT NULL,
   foreign key (package) references package(name),
   primary key(url)
);
CREATE TABLE IF NOT EXISTS run (
   id text not null primary key,
   command text,
   description text,
   start_time timestamp,
   finish_time timestamp,
   package text not null,
   -- Associated merge proposal URL, if any.
   merge_proposal_url text null,
   -- Debian version text of the built package
   build_version text,
   -- Distribution the package was built for (e.g. "lintian-fixes")
   build_distribution text,
   result_code text,
   -- Some subworker-specific indication of what we attempted to do
   context text,
   -- Main branch revision
   main_branch_revision text,
   foreign key (package) references package(name),
   foreign key (merge_proposal_url) references merge_proposal(url)
);
CREATE TYPE publish_mode AS ENUM('push', 'attempt-push', 'propose', 'build-only');
CREATE TABLE IF NOT EXISTS queue (
   id serial,
   package text not null,
   command text not null,
   committer text null,
   mode publish_mode not null,
   priority integer default 0 not null,
   foreign key (package) references package(name),
   -- Some subworker-specific indication of what we are expecting to do.
   context text,
   unique(package, command, mode)
);
