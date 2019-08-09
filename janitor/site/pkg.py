#!/usr/bin/python3

from io import BytesIO

from breezy.errors import NotBranchError
from janitor import state
from janitor.build import (
    changes_filename,
)
from janitor.sbuild_log import (
    parse_sbuild_log,
    find_failed_stage,
    find_build_failure_description,
    SBUILD_FOCUS_SECTION,
    strip_useless_build_tail,
)
from janitor.site import (
    changes_get_binaries,
    env,
    get_build_architecture,
    open_changes_file,
    get_run_diff,
    highlight_diff,
)
from janitor.vcs import (
    CACHE_URL_BZR,
    CACHE_URL_GIT,
    get_vcs_abbreviation,
)

FAIL_BUILD_LOG_LEN = 15

BUILD_LOG_NAME = 'build.log'
WORKER_LOG_NAME = 'worker.log'


def find_build_log_failure(logf, length):
    offsets = {}
    linecount = 0
    paragraphs = {}
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
    else:
        include_lines = (1, linecount)
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


async def generate_run_file(logfile_manager, vcs_manager, run):
    (start_time, finish_time) = run.times
    kwargs = {}
    kwargs['run_id'] = run.id
    kwargs['command'] = run.command
    kwargs['description'] = run.description
    kwargs['package'] = run.package
    kwargs['start_time'] = run.times[0]
    kwargs['finish_time'] = run.times[1]
    kwargs['build_version'] = run.build_version
    kwargs['build_distribution'] = run.build_distribution
    kwargs['result_code'] = run.result_code
    kwargs['result'] = run.result
    kwargs['branch_name'] = run.branch_name
    kwargs['revision'] = run.revision
    kwargs['enumerate'] = enumerate
    kwargs['branch_url'] = run.branch_url

    package = await state.get_package(run.package)
    kwargs['vcs_browse'] = package.vcs_browse
    kwargs['vcs_url'] = package.vcs_url

    def show_diff():
        diff = get_run_diff(vcs_manager, run)
        if diff is None:
            return None
        return diff.decode('utf-8', 'replace')
    kwargs['show_diff'] = show_diff
    kwargs['highlight_diff'] = highlight_diff
    kwargs['max'] = max
    kwargs['suite'] = run.suite

    def read_file(f):
        return [l.decode('utf-8', 'replace') for l in f.readlines()]
    kwargs['read_file'] = read_file
    if run.build_version:
        kwargs['changes_name'] = changes_filename(
            run.package, run.build_version,
            get_build_architecture())
    else:
        kwargs['changes_name'] = None
    try:
        repo = vcs_manager.get_repository(run.package)
    except NotBranchError:
        repo = None
    if repo:
        kwargs['vcs'] = get_vcs_abbreviation(repo)
    else:
        kwargs['vcs'] = None
    kwargs['cache_url_git'] = CACHE_URL_GIT
    kwargs['cache_url_bzr'] = CACHE_URL_BZR
    kwargs['in_line_boundaries'] = in_line_boundaries
    if kwargs['changes_name']:
        try:
            changes_file = open_changes_file(run, kwargs['changes_name'])
        except FileNotFoundError:
            pass
        else:
            kwargs['binary_packages'] = []
            for binary in changes_get_binaries(changes_file):
                kwargs['binary_packages'].append(binary)

    cached_logs = {}

    async def _cache_log(name):
        try:
            cached_logs[name] = (await logfile_manager.get_log(
                run.package, run.id, name)).read()
        except FileNotFoundError:
            cached_logs[name] = None

    async def has_log(name):
        if name not in cached_logs:
            await _cache_log(name)
        return cached_logs[name] is not None

    async def get_log(name):
        if name not in cached_logs:
            await _cache_log(name)
        if cached_logs[name] is None:
            raise FileNotFoundError(name)
        return BytesIO(cached_logs[name])
    kwargs['get_log'] = lambda name: BytesIO(cached_logs.get(name, b''))
    if await has_log(BUILD_LOG_NAME):
        kwargs['build_log_name'] = BUILD_LOG_NAME
        kwargs['earlier_build_log_names'] = []
        i = 1
        while await has_log(BUILD_LOG_NAME + '.%d' % i):
            log_name = '%s.%d' % (BUILD_LOG_NAME, i)
            kwargs['earlier_build_log_names'].append((i, log_name))
            i += 1

        logf = await get_log(BUILD_LOG_NAME)
        line_count, include_lines, highlight_lines = find_build_log_failure(
            logf, FAIL_BUILD_LOG_LEN)
        kwargs['build_log_line_count'] = line_count
        kwargs['build_log_include_lines'] = include_lines
        kwargs['build_log_highlight_lines'] = highlight_lines

    if await has_log(WORKER_LOG_NAME):
        kwargs['worker_log_name'] = WORKER_LOG_NAME

    template = env.get_template('run.html')
    text = await template.render_async(**kwargs)
    return text


async def generate_pkg_file(package, merge_proposals, runs):
    kwargs = {}
    kwargs['package'] = package.name
    kwargs['maintainer_email'] = package.maintainer_email
    kwargs['vcs_url'] = package.vcs_url
    kwargs['vcs_browse'] = package.vcs_browse
    kwargs['merge_proposals'] = merge_proposals
    kwargs['builds'] = [run for run in runs if run.build_version]
    kwargs['runs'] = runs
    kwargs['candidates'] = {
        suite: (context, value)
        for (package, suite, command, context, value) in
        await state.iter_candidates(packages=[package.name])}
    template = env.get_template('package-overview.html')
    return await template.render_async(**kwargs)


async def generate_pkg_list(packages):
    template = env.get_template('package-name-list.html')
    return await template.render_async(
        packages=[name for (name, maintainer) in packages])


async def generate_maintainer_list(packages):
    template = env.get_template('by-maintainer-package-list.html')
    by_maintainer = {}
    for name, maintainer in packages:
        by_maintainer.setdefault(maintainer, []).append(name)
    return await template.render_async(by_maintainer=by_maintainer)


async def generate_ready_list(suite):
    template = env.get_template('ready-list.html')
    runs = list(await state.iter_publish_ready(suite=suite))
    return await template.render_async(runs=runs, suite=suite)
