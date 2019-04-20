CREATE TABLE IF NOT EXISTS package (
   name string not null,
   branch_url string not null,
   maintainer_email string not null,
   primary key(name), unique(branch_url)
);
CREATE TABLE IF NOT EXISTS merge_proposal (
   package string,
   url string not null,
   status string check(status in ("open", "closed", "merged")) NULL DEFAULT NULL,
   foreign key (package) references package(name),
   primary key(url)
);
CREATE TABLE IF NOT EXISTS run (
   id string not null primary key,
   command string,
   description string,
   start_time string,
   finish_time string,
   package string not null,
   merge_proposal_url string null,
   changes_filename string,
   build_distribution string,
   foreign key (package) references package(name),
   foreign key (merge_proposal_url) references merge_proposal(url)
);
CREATE TABLE IF NOT EXISTS queue (
   id integer primary key autoincrement,
   package string not null,
   command string not null,
   committer string null,
   mode string check(mode in ("push", "attempt-push", "propose", "build-only")) not null,
   foreign key (package) references package(name),
   unique(package, command, mode)
);
