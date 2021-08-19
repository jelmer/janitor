#!/usr/bin/python3

import tdb
import json
import os
from urllib.request import urlopen
import subprocess
from tempfile import TemporaryDirectory
from os import O_RDWR, O_CREAT
import sys

check = tdb.open('checked.tdb', flags=O_RDWR | O_CREAT)

runs = {}
with urlopen('https://janitor.debian.net/api/result-codes/upstream-branch-unknown') as f:
    for entry in json.load(f):
        runs[entry['package']] = (entry['vcs_type'], entry['branch_url'])

for package, (vcs_type, url) in runs.items():
    if package.encode('utf-8') in check:
        continue
    print('package: %s (%s)' % (package, url))
    with TemporaryDirectory() as td:
        try:
            subprocess.check_call(['brz', 'clone', url, td])
            grep = subprocess.check_output(['brz', 'grep', '-Xdebian/*', '(github.com|bitbucket.org|gitlab.com|gitlab|sf.net)'], cwd=td)
            grep += subprocess.check_output(['brz', 'grep', '-Xdebian/*', '(CVS|Subversion|svn|SVN|Git|Bazaar|bzr|fossil)'], cwd=td)
            sys.stdout.buffer.write(grep)
            if grep:
                subprocess.check_call(['/bin/bash'], cwd=td)
                result = "yes"
            else:
                result = "nothing"
        except subprocess.CalledProcessError:
            result = "error"
        check[package.encode('utf-8')] = result.encode()
