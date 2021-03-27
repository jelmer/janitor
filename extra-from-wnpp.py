#!/usr/bin/python3

import debianbts
import logging
import re
import sys

logging.basicConfig(level=logging.INFO)


from janitor.upstream_project_pb2 import ExtraUpstreamProjects

for bug in sorted(debianbts.get_bugs(package='wnpp', status='open'), reverse=True):
    (status, ) = debianbts.get_status(bug)
    (kind, title) = status.subject.split(':', 1)
    if kind not in ('ITP', 'RFP'):
        logging.debug('Skipping %d, not ITP/RFP: %s', bug, status.subject)
        continue
    log = debianbts.get_bug_log(bug)
    body = log[0]['message'].get_payload()
    while isinstance(body, list):
        body = body[0].get_payload()
    url = None
    name = None
    for line in body.splitlines(False):
        if line.startswith('* '):
            line = line[2:]
        m = re.fullmatch('\s*URL\s*:\s*(.*)', line)
        if m:
            url = m.group(1)
        m = re.fullmatch('\s*Package name\s*:\s*(.*)', line)
        if m:
            name = m.group(1)

    if not name:
        logging.debug('Skipping %d, missing name', bug)
        continue

    if not url:
        logging.debug('Skipping %d, missing url', bug)
        continue

    print('# Bug %d: %s' % (bug, status.subject))
    pl = ExtraUpstreamProjects()
    project = pl.upstream_project.add()
    project.name = name
    project.vcs_url = url
    project.vcs_type = "Git"
    print(pl)
    sys.stdout.flush()
