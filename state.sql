CREATE TABLE IF NOT EXISTS package (
   id integer primary key autoincrement,
   name string not null,
   branch_url string not null,
   maintainer_email string not null,
   unique(branch_url), unique(name)
);
CREATE TABLE IF NOT EXISTS merge_proposal (
   id integer primary key autoincrement,
   package_id integer,
   url string not null,
   foreign key (package_id) references package(id)
   unique(url)
);
CREATE TABLE IF NOT EXISTS run (
   id string primary key,
   command string,
   description string,
   start_time interger,
   finish_time integer,
   package_id integer,
   merge_proposal_id integer,
   foreign key (package_id) references package(id),
   foreign key (merge_proposal_id) references merge_proposal(id)
);
