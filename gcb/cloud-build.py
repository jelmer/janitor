#!/usr/bin/python3

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.parse
import urllib3


BUCKET_NAME = 'results.janitor.debian.net'

# In theory, we should be able to use pubsub to be notified when our build finishes..
#     subscriber = pubsub.SubscriberClient()
#    topic = 'projects/debian-janitor/topics/cloud-builds'
#    subscription_name = 'projects/debian-janitor/subscriptions/worker'
#    subscription = subscriber.create_subscription(
#        subscription_name, topic)
#    def callback(message):
#        import pdb; pdb.set_trace()
#        message.ack()
#    future = subscription.open(callback)
#    future.result()


def gcb_run_build(http, bearer, args):
    env = {}
    for key in ['PACKAGE', 'COMMITTER']:
        if key in os.environ:
            env[key] = os.environ[key]
    request = {
        "steps": [{
            "name": "gcr.io/$PROJECT_ID/worker",
            "args": [
                '--build-command='
                'sbuild -A -s -v -d$$DISTRIBUTION -c unstable-amd64-sbuild',
                ] + args,
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


def get_blob(http, bearer, url):
    result = urllib.parse.urlparse(url)
    r = http.request(
        'GET',
        "https://www.googleapis.com/storage/v1/b/%(bucket_id)s/o/%(object_name)s?alt=media" % {
            'object_name': urllib.parse.quote(result.path.lstrip('/'), ''),
            'bucket_id': result.netloc,
        },
        headers={'Authorization': "Bearer %s" % bearer})
    return r


def download_results(http, bearer, manifest_url, output_directory):
    blob = get_blob(http, bearer, manifest_url)
    for line in blob.data.splitlines():
        manifest = json.loads(line)
        blob = get_blob(http, bearer, manifest['location'])
        name = urllib.parse.urlparse(manifest['location']).path
        path = os.path.join(output_directory, os.path.basename(name))
        with open(path, 'wb') as f:
            f.write(blob.data)


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

    while True:
        # Urgh
        time.sleep(10)
        r = http.request(
            'GET',
            'https://cloudbuild.googleapis.com/v1/projects/debian-janitor/builds/%s' % build_id,
            headers={'Authorization': "Bearer %s" % bearer})
        ops = json.loads(r.data.decode('utf-8'))
        if ops['status'] == 'SUCCESS':
            break
    artifact_manifest_url = ops['results']['artifactManifest']
    download_results(
        http, bearer, artifact_manifest_url, args.output_directory)
    blob = get_blob(http, bearer, ops['logsBucket'] + '/log-%s.txt' % build_id)
    sys.stdout.buffer.write(blob.data)
    tgz_name = os.environ['PACKAGE'] + '.tgz'
    if os.path.exists(os.path.join(args.output_directory, tgz_name)):
        subprocess.check_call(
            ['tar', 'xfz', tgz_name],
            cwd=args.output_directory)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
