#!/usr/bin/python3

import argparse
import json
import os
import subprocess
import sys
import urllib3


def gcb_run_build(args):
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
             'location': 'gs://my-bucket/artifacts/',
             'paths': ["result.json", "build.log",
                       "*.dsc", "*.changes", "*.deb", "*.tar.*"],
          }
        }
    }

    bearer = subprocess.check_output(
        ["gcloud", "config", "config-helper",
         "--format=value(credential.access_token)"])
    http = urllib3.PoolManager()
    r = http.request(
        'POST',
        'https://cloudbuild.googleapis.com/v1/projects/debian-janitor/builds',
        body=json.dumps(request),
        headers={'Authorization': "Bearer %s" % bearer.decode().strip("\n")})
    response = json.loads(r.data.decode('utf-8'))
    print("Log URL: %s" % response['metadata']['build']['logUrl'])
    build_id = response['metadata']['build']['id']
    import pdb; pdb.set_trace()


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog='janitor-worker',
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument(
        '--output-directory', type=str,
        help='Output directory', default='.')
    args, unknown = parser.parse_known_args()

    gcb_run_build(unknown)

    # TODO(jelmer): Copy artefacts to output-directory


if __name__ == '__main__':
    sys.exit(main(sys.argv))
