CREATE TABLE IF NOT EXISTS package (
   name text not null,
   branch_url text not null,
   maintainer_email text not null,
   uploader_emails text[] not null,
   primary key(name)
);
CREATE TYPE merge_proposal_status AS ENUM ('open', 'closed', 'merged');
CREATE TABLE IF NOT EXISTS merge_proposal (
   package text,
   url text not null,
   status merge_proposal_status NULL DEFAULT NULL,
   revision text,
   foreign key (package) references package(name),
   primary key(url)
);
CREATE TYPE suite AS ENUM('lintian-fixes', 'fresh-releases', 'fresh-snapshots');
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
   build_version debversion,
   -- Distribution the package was built for (e.g. "lintian-fixes")
   build_distribution text,
   result_code text,
   instigated_context text,
   -- Some subworker-specific indication of what we attempted to do
   context text,
   -- Main branch revision
   main_branch_revision text,
   branch_name text,
   revision text,
   result json,
   suite suite not null,
   foreign key (package) references package(name),
   foreign key (merge_proposal_url) references merge_proposal(url)
);
CREATE TYPE publish_mode AS ENUM('push', 'attempt-push', 'propose', 'build-only', 'push-derived');
CREATE TABLE IF NOT EXISTS publish (
   package text not null,
   branch_name text,
   main_branch_revision text,
   revision text,
   mode publish_mode not null,
   merge_proposal_url text,
   result_code text,
   description text,
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
   priority integer default 0 not null,
   foreign key (package) references package(name),
   -- Some subworker-specific indication of what we are expecting to do.
   context text,
   estimated_duration interval,
   unique(package, command)
);
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
   command text not null,
   context text,
   value integer,
   foreign key (package) references package(name)
);
