#!/usr/bin/python3

from debian.deb822 import Changes
import os

from janitor import state
from janitor.build import (
    changes_filename,
    get_build_architecture,
)
from janitor.sbuild_log import (
    parse_sbuild_log,
    find_failed_stage,
    find_build_failure_description,
    SBUILD_FOCUS_SECTION,
    strip_useless_build_tail,
)
from janitor.site import (
    format_duration,
)
from janitor.site import env
from janitor.trace import note, warning
from janitor.vcs import (
    CACHE_URL_BZR,
    CACHE_URL_GIT,
)

FAIL_BUILD_LOG_LEN = 15

def changes_get_binaries(changes_path):
    with open(changes_path, "r") as cf:
        changes = Changes(cf)
        return changes['Binary'].split(' ')


def find_build_log_failure(log_path, length):
    offsets = {}
    linecount = 0
    paragraphs = {}
    with open(log_path, 'rb') as logf:
        for title, offset, lines in parse_sbuild_log(logf):
            if title is not None:
                title = title.lower()
            paragraphs[title] = lines
            linecount = max(offset[1], linecount)
            offsets[title] = offset
    highlight_lines = []
    include_lines = None
    failed_stage = find_failed_stage(paragraphs.get('summary', []))
    focus_section = SBUILD_FOCUS_SECTION.get(failed_stage)
    if focus_section not in paragraphs:
        focus_section = None
    if focus_section:
        include_lines = (max(1, offsets[focus_section][1]-length),
                         offsets[focus_section][1])
    elif length < linecount:
        include_lines = (linecount-length, None)
    if focus_section == 'build':
        lines = paragraphs.get(focus_section, [])
        lines = strip_useless_build_tail(lines)
        include_lines = (max(1, offsets[focus_section][0] + len(lines)-length),
                         offsets[focus_section][0] + len(lines))
        offset, unused_line, unused_err = find_build_failure_description(lines)
        if offset is not None:
            highlight_lines = [offsets[focus_section][0] + offset]

    return (linecount, include_lines, highlight_lines)


def in_line_boundaries(i, boundaries):
    if boundaries is None:
        return True
    if boundaries[0] is not None and i < boundaries[0]:
        return False
    if boundaries[1] is not None and i > boundaries[1]:
        return False
    return True


async def write_run_file(logdirectory, dir, run_id, times, command, description,
            package_name, merge_proposal_url, build_version,
            build_distro, result_code, branch_name):
    (start_time, finish_time) = times

    run_dir = os.path.join(dir, package_name, run_id)
    os.makedirs(run_dir, exist_ok=True)

    kwargs = {}
    kwargs['run_id'] = run_id
    kwargs['kind'] = command.split(' ')[0]
    kwargs['command'] = command
    kwargs['description'] = description
    kwargs['package'] = package_name
    kwargs['start_time'] = start_time
    kwargs['finish_time'] = finish_time
    kwargs['merge_proposal_url'] = merge_proposal_url
    kwargs['build_version'] = build_version
    kwargs['build_distro'] = build_distro
    kwargs['result_code'] = result_code
    kwargs['branch_name'] = branch_name
    kwargs['format_duration'] = format_duration
    kwargs['enumerate'] = enumerate
    kwargs['max'] = max

    def read_file(p):
        with open(p, 'rb') as f:
            return [l.decode('utf-8', 'replace') for l in f.readlines()]
    kwargs['read_file'] = read_file
    if build_version:
        kwargs['changes_name'] = changes_filename(
            package_name, build_version,
            get_build_architecture())
    else:
        kwargs['changes_name'] = None
    if os.path.exists('../vcs/git/%s' % package_name):
        kwargs['vcs'] = 'git'
    elif os.path.exists('../vcs/bzr/%s' % package_name):
        kwargs['vcs'] = 'bzr'
    else:
        kwargs['vcs'] = None
    kwargs['cache_url_git'] = CACHE_URL_GIT
    kwargs['cache_url_bzr'] = CACHE_URL_BZR
    kwargs['binary_packages'] = []
    kwargs['in_line_boundaries'] = in_line_boundaries
    if kwargs['changes_name']:
        changes_path = os.path.join(
            "../public_html", build_distro, kwargs['changes_name'])
        if not os.path.exists(changes_path):
            warning('Missing changes path %r', changes_path)
        else:
            for binary in changes_get_binaries(changes_path):
                kwargs['binary_packages'].append(binary)

    build_log_name = 'build.log'
    worker_log_name = 'worker.log'
    log_directory = os.path.join(logdirectory, package_name, run_id)
    build_log_path = os.path.join(log_directory, build_log_name)
    if os.path.exists(build_log_path):
        if not os.path.exists(os.path.join(run_dir, build_log_name)):
            os.symlink(build_log_path, os.path.join(run_dir, build_log_name))
        kwargs['build_log_name'] = build_log_name
        kwargs['build_log_path'] = build_log_path
        kwargs['earlier_build_log_names'] = []
        i = 1
        while os.path.exists(os.path.join(
                log_directory, build_log_name + '.%d' % i)):
            log_name = '%s.%d' % (build_log_name, i)
            kwargs['earlier_build_log_names'].append((i, log_name))
            if not os.path.exists(os.path.join(run_dir, log_name)):
                os.symlink(os.path.join(log_directory, log_name),
                           os.path.join(run_dir, log_name))
            i += 1

        line_count, include_lines, highlight_lines = find_build_log_failure(
            build_log_path, FAIL_BUILD_LOG_LEN)
        kwargs['build_log_line_count'] = line_count
        kwargs['build_log_include_lines'] = include_lines
        kwargs['build_log_highlight_lines'] = highlight_lines

    worker_log_path = os.path.join(log_directory, worker_log_name)
    if os.path.exists(worker_log_path):
        if not os.path.exists(os.path.join(run_dir, worker_log_name)):
            os.symlink(worker_log_path, os.path.join(run_dir, worker_log_name))
        kwargs['worker_log_name'] = worker_log_name
        kwargs['worker_log_path'] = worker_log_path

    with open(os.path.join(run_dir, 'index.html'), 'w') as f:
        template = env.get_template('run.html')
        f.write(await template.render_async(**kwargs))
    note('Wrote %s', run_dir)


async def write_run_files(logdirectory, dir):
    runs_by_pkg = {}

    jobs = []
    async for run in state.iter_runs():
        package_name = run[4]
        jobs.append(write_run_file(logdirectory, dir, *run))
        runs_by_pkg.setdefault(package_name, []).append(run)
    await asyncio.gather(*jobs)

    return runs_by_pkg


async def write_pkg_file(dir, name, merge_proposals, maintainer_email, branch_url,
                         runs):
    pkg_dir = os.path.join(dir, name)
    if not os.path.isdir(pkg_dir):
        os.mkdir(pkg_dir)

    kwargs = {}
    kwargs['package'] = name
    kwargs['maintainer_email'] = maintainer_email
    kwargs['vcs_url'] = branch_url
    kwargs['merge_proposals'] = merge_proposals
    kwargs['builds'] = [run for run in runs if run[6]]
    kwargs['runs'] = runs

    with open(os.path.join(pkg_dir, 'index.html'), 'w') as f:
        template = env.get_template('package-overview.html')
        f.write(await template.render_async(**kwargs))


async def write_pkg_files(dir):
    merge_proposals = {}
    for package, url, status in await state.iter_proposals():
        merge_proposals.setdefault(package, []).append((url, status))

    jobs = []
    packages = []
    for (name, maintainer_email, branch_url) in await state.iter_packages():
        packages.append(name)
        jobs.append(write_pkg_file(
            dir, name, merge_proposals.get(name, []),
            maintainer_email, branch_url, runs_by_pkg.get(name, [])))

    await asyncio.gather(*jobs)

    return packages


async def write_pkg_list(dir, packages):
    with open(os.path.join(dir, 'index.html'), 'w') as f:
        template = env.get_template('package-name-list.html')
        f.write(await template.render_async(packages=packages))


if __name__ == '__main__':
    import argparse
    import asyncio
    parser = argparse.ArgumentParser(prog='report-pkg')
    parser.add_argument("logdirectory")
    parser.add_argument("directory")
    args = parser.parse_args()
    if not os.path.isdir(args.directory):
        os.mkdir(args.directory)
    loop = asyncio.get_event_loop()
    runs_by_pkg = loop.run_until_complete(write_run_files(args.logdirectory, args.directory))
    packages = loop.run_until_complete(write_pkg_files(args.directory))
    loop.run_until_complete(write_pkg_list(args.directory, packages))
