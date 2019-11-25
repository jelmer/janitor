#!/usr/bin/python3

import argparse
import shlex
import subprocess
import os
import sys
import asyncio
import asyncssh


DEFAULT_JANITOR_GIT_URL = 'https://salsa.debian.org/jelmer/debian-janitor'


async def run_ssh_worker(hostname, package, logf, output_directory, args,
                         timeout=None):
    async with asyncssh.connect(hostname) as conn:
        result = await conn.run('mktemp -d', check=True)
        remote_output_directory = result.stdout.strip()
        remote_janitor_dir = '$HOME/debian-janitor'
        git_command = """\
if test -d %s
then
  git -C %s pull --recurse-submodules
else
  git clone --recursive %s %s
fi
""" % (
            remote_janitor_dir, remote_janitor_dir, DEFAULT_JANITOR_GIT_URL,
            remote_janitor_dir)
        await conn.run(git_command, check=True)
        pythonpath = [
            remote_janitor_dir,
            os.path.join(remote_janitor_dir, 'lintian-brush'),
            os.path.join(remote_janitor_dir, 'silver-platter'),
            os.path.join(remote_janitor_dir, 'breezy'),
        ]
        args = ([
             'PACKAGE=%s' % package,
             'PYTHONPATH=' + ':'.join(pythonpath),
             'python3',
             '-m', 'janitor.worker',
             '--output-directory=%s' % remote_output_directory,
             '--tgz-repo'] + args)
        worker_command = ' '.join([shlex.quote(arg) for arg in args])
        try:
            await conn.run(
                worker_command, stdout=logf, stderr=logf, check=True)
            await asyncssh.scp(
                [(conn, os.path.join(remote_output_directory, pattern))
                 for pattern in ['*.json', '*.tgz', '*.log']],
                output_directory)
            tgz_name = package + '.tgz'
            tgz_path = os.path.join(output_directory, tgz_name)
            if os.path.exists(tgz_path):
                subprocess.check_call(
                    ['tar', 'xfz', tgz_name], cwd=output_directory)
                os.unlink(tgz_path)
        finally:
            await conn.run('rm -rf %s' % shlex.quote(remote_output_directory))


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog='ssh-worker',
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    parser.add_argument(
        '--output-directory', type=str,
        help='Output directory', default='.')
    parser.add_argument(
        '--timeout', type=int,
        help='Build timeout (in seconds)', default=3600)
    parser.add_argument(
        '--host', type=str, default='localhost',
        help='Host to connect to.')
    args, unknown = parser.parse_known_args()

    asyncio.run(run_ssh_worker(
        args.host, os.environ['PACKAGE'],
        sys.stdout.buffer, args.output_directory, unknown, args.timeout))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
