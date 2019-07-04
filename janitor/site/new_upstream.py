#!/usr/bin/python3

import argparse
import asyncio
import os
import sys

from janitor import state
from janitor.site import env


async def generate_pkg_file(package, suite):
    try:
        (package, maintainer_email, vcs_url) = list(await state.iter_packages(package=package))[0]
    except IndexError:
        raise KeyError(package)
    # TODO(jelmer): Filter out proposals not for this suite.
    merge_proposals = [
        (url, status)
        for (package, url, status) in await state.iter_proposals(package)]
    (command, build_version, result_code,
     context, start_time, run_id, result) = await state.get_last_success(package, suite)
    kwargs = {
        'package': package,
        'merge_proposals': merge_proposals,
        'maintainer_email': maintainer_email,
        'vcs_url': vcs_url,
        'command': command,
        'build_version': build_version,
        'result_code': result_code,
        'context': context,
        'start_time': start_time,
        'run_id': run_id,
        'result': result,
        'suite': suite,
        }
    template = env.get_template('new-upstream-package.html')
    return await template.render_async(**kwargs)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='report-new-upstream-pkg')
    parser.add_argument("package")
    args = parser.parse_args()

    loop = asyncio.get_event_loop()
    sys.stdout.write(loop.run_until_complete(generate_pkg_file(args.package)))
