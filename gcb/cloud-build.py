#!/usr/bin/python3

import argparse
import json
import os
import subprocess
import sys
import urllib3


def gcb_run_build(http, bearer, args):
    env = {}
    for key in ['PACKAGE', 'COMMITTER']:
        if key in os.environ:
            env[key] = os.environ[key]
    request = {
        "steps": [{
            "name": "gcr.io/$PROJECT_ID/worker",
            "args": ['--output-directory=/workspace'] + args,
            "env": ['%s=%s' % item for item in env.items()],
        }],
        "artifacts": {
          'objects': {
             'location': 'gs://results.janitor.debian.net/$BUILD_ID',
             'paths': ["*"],
          }
        }
    }

    r = http.request(
        'POST',
        'https://cloudbuild.googleapis.com/v1/projects/debian-janitor/builds',
        body=json.dumps(request),
        headers={'Authorization': "Bearer %s" % bearer})
    response = json.loads(r.data.decode('utf-8'))
    print("Log URL: %s" % response['metadata']['build']['logUrl'])
    build_id = response['metadata']['build']['id']
    return build_id


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog='janitor-worker',
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument(
        '--output-directory', type=str,
        help='Output directory', default='.')
    args, unknown = parser.parse_known_args()

    http = urllib3.PoolManager()
    bearer = subprocess.check_output(
        ["gcloud", "config", "config-helper",
         "--format=value(credential.access_token)"]).decode().strip("\n")

    build_id = gcb_run_build(http, bearer, unknown)
    r = http.request(
        'GET',
        'https://cloudbuild.googleapis.com/v1/projects/debian-janitor/builds/%s' % build_id,
        headers={'Authorization': "Bearer %s" % bearer})
    build_state = json.loads(r.data.decode('utf-8'))
    # TODO(jelmer): Copy artefacts to output-directory


if __name__ == '__main__':
    sys.exit(main(sys.argv))
