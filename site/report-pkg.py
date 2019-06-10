#!/usr/bin/python3

import argparse
from debian.deb822 import Changes
import os
import sys
sys.path.insert(0, os.path.dirname(__file__))

from janitor import state  # noqa: E402
from janitor.build import (
    changes_filename,
    get_build_architecture,
)  # noqa: E402
from janitor.sbuild_log import (
    parse_sbuild_log,
    find_failed_stage,
    find_build_failure_description,
    SBUILD_FOCUS_SECTION,
    strip_useless_build_tail,
)  # noqa: E402
from janitor.site import (
    format_duration,
)  # noqa: E402
from janitor.trace import warning  # noqa: E402
from janitor.vcs import (
    CACHE_URL_BZR,
    CACHE_URL_GIT,
)  # noqa: E402

from jinja2 import Environment, FileSystemLoader, select_autoescape
env = Environment(
    loader=FileSystemLoader('templates'),
    autoescape=select_autoescape(['html', 'xml'])
)


FAIL_BUILD_LOG_LEN = 15

parser = argparse.ArgumentParser(prog='report-pkg')
parser.add_argument("logdirectory")
parser.add_argument("directory")
args = parser.parse_args()
dir = args.directory


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


if not os.path.isdir(dir):
    os.mkdir(dir)


runs_by_pkg = {}

for run in state.iter_runs():
    (run_id, (start_time, finish_time), command, description,
        package_name, merge_proposal_url, build_version,
        build_distro, result_code, branch_name) = run

    runs_by_pkg.setdefault(package_name, []).append(run)

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
            return list(f.readlines())
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
    log_directory = os.path.join(args.logdirectory, package_name, run_id)
    build_log_path = os.path.join(log_directory, build_log_name)
    if os.path.exists(build_log_path):
        kwargs['build_log_filename'] = build_log_name
        kwargs['build_log_path'] = build_log_path
        kwargs['earlier_build_log_names'] = []
        i = 1
        while os.path.exists(os.path.join(
                log_directory, build_log_name + '.%d' % i)):
            kwargs['earlier_build_log_names'].append(
                (i, '%s.%d' % (build_log_name, i)))
            i += 1

        line_count, include_lines, highlight_lines = find_build_log_failure(
            build_log_path, FAIL_BUILD_LOG_LEN)
        kwargs['build_log_line_count'] = line_count
        kwargs['build_log_include_lines'] = include_lines
        kwargs['build_log_highlight_lines'] = highlight_lines

    worker_log_path = os.path.join(log_directory, worker_log_name)
    if os.path.exists(worker_log_path):
        kwargs['worker_log_name'] = worker_log_name
        kwargs['worker_log_path'] = worker_log_path

    with open(os.path.join(run_dir, 'index.html'), 'w') as f:
        template = env.get_template('run.html')
        f.write(template.render(**kwargs))


merge_proposals = {}
for package, url, status in state.iter_proposals():
    merge_proposals.setdefault(package, []).append((url, status))


packages = []
for (name, maintainer_email, branch_url) in state.iter_packages():
    packages.append(name)

    pkg_dir = os.path.join(dir, name)
    if not os.path.isdir(pkg_dir):
        os.mkdir(pkg_dir)

    kwargs = {}
    kwargs['package'] = name
    kwargs['maintainer_email'] = maintainer_email
    kwargs['vcs_url'] = branch_url
    kwargs['merge_proposals'] = merge_proposals.get(name, [])
    kwargs['builds'] = [run for run in runs_by_pkg.get(name, []) if run[6]]
    kwargs['runs'] = runs_by_pkg.get(name, [])

    with open(os.path.join(pkg_dir, 'index.html'), 'w') as f:
        template = env.get_template('package-overview.html')
        f.write(template.render(**kwargs))


with open(os.path.join(dir, 'index.html'), 'w') as f:
    template = env.get_template('package-name-list.html')
    f.write(template.render(packages=packages))
