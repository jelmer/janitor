#!/usr/bin/python3

import sqlite3
import sys

con = sqlite3.connect(sys.argv[1])

print("Merge Proposals")
print("===============")

cur = con.cursor()
cur.execute(
    "SELECT branch.url, merge_proposal.url FROM branch "
    "LEFT JOIN merge_proposal ON branch.id = merge_proposal.branch_id")
for (branch_url, mp_url) in cur.fetchall():
    print("* %s" % branch_url)
    if mp_url:
        print(" - %s" % mp_url)
    print("")

print("Recent Runs")
print("===========")
cur.execute(
    "SELECT run.command, run.finish_time, merge_proposal.url FROM run "
    "LEFT JOIN merge_proposal ON run.merge_proposal_id = merge_proposal.id")

for (command, finish_time, merge_proposal_url) in cur.fetchall():
    print("* %s %s %s" % (command, finish_time, merge_proposal_url))
