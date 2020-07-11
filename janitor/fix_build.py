#!/usr/bin/python
# Copyright (C) 2018 Jelmer Vernooij <jelmer@jelmer.uk>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA

__all__ = [
    'build_incrementally',
]

import os
import re
import subprocess
import sys

from debian.deb822 import (
    Deb822,
    PkgRelation,
    )

from breezy.commit import PointlessCommit
from lintian_brush import (
    reset_tree,
    )
from lintian_brush.changelog import (
    add_changelog_entry,
    )
from debmutate.control import (
    ensure_some_version,
    ensure_minimum_version,
    pg_buildext_updatecontrol,
    ControlEditor,
    )
from debmutate.debhelper import (
    get_debhelper_compat_level,
    )
from debmutate.deb822 import (
    Deb822Editor,
    )
from debmutate.reformatting import (
    FormattingUnpreservable,
    GeneratedFile,
    )
from lintian_brush.rules import (
    dh_invoke_add_with,
    update_rules,
    )
from silver_platter.debian import (
    debcommit,
    DEFAULT_BUILDER,
    )

from .build import attempt_build
from .trace import note, warning
from .sbuild_log import (
    MissingConfigStatusInput,
    MissingPythonModule,
    MissingPythonDistribution,
    MissingCHeader,
    MissingPkgConfig,
    MissingCommand,
    MissingFile,
    MissingJavaScriptRuntime,
    MissingSprocketsFile,
    MissingGoPackage,
    MissingPerlFile,
    MissingPerlModule,
    MissingXmlEntity,
    MissingJDKFile,
    MissingNodeModule,
    MissingPhpClass,
    MissingRubyGem,
    MissingLibrary,
    MissingJavaClass,
    MissingConfigure,
    MissingAutomakeInput,
    MissingRPackage,
    MissingRubyFile,
    MissingAutoconfMacro,
    MissingValaPackage,
    NeedPgBuildExtUpdateControl,
    SbuildFailure,
    DhAddonLoadFailure,
    AptFetchFailure,
    MissingMavenArtifacts,
    GnomeCommonMissing,
    )


DEFAULT_MAX_ITERATIONS = 10


class CircularDependency(Exception):
    """Adding dependency would introduce cycle."""

    def __init__(self, package):
        self.package = package


def add_dependency(
        tree, context, package, minimum_version=None, committer=None,
        subpath='', update_changelog=True):
    if context == ('build', ):
        return add_build_dependency(
            tree, package, minimum_version=minimum_version,
            committer=committer, subpath=subpath,
            update_changelog=update_changelog)
    elif context[0] == 'autopkgtest':
        return add_test_dependency(
            tree, context[1], package, minimum_version=minimum_version,
            committer=committer, subpath=subpath,
            update_changelog=update_changelog)
    else:
        raise AssertionError('context %r invalid' % context)


def add_build_dependency(tree, package, minimum_version=None,
                         committer=None, subpath='', update_changelog=True):
    if not isinstance(package, str):
        raise TypeError(package)

    control_path = os.path.join(tree.abspath(subpath), 'debian/control')
    try:
        with ControlEditor(path=control_path) as updater:
            for binary in updater.binaries:
                if binary["Package"] == package:
                    raise CircularDependency(package)
            if minimum_version:
                updater.source["Build-Depends"] = ensure_minimum_version(
                    updater.source.get("Build-Depends", ""),
                    package, minimum_version)
            else:
                updater.source["Build-Depends"] = ensure_some_version(
                    updater.source.get("Build-Depends", ""), package)
    except FormattingUnpreservable as e:
        note('Unable to edit %s in a way that preserves formatting.',
             e.path)
        return False

    if minimum_version:
        desc = "%s (>= %s)" % (package, minimum_version)
    else:
        desc = package

    if not updater.changed:
        note('Giving up; dependency %s was already present.', desc)
        return False

    note("Adding build dependency: %s", desc)
    return commit_debian_changes(
        tree, subpath, "Add missing build dependency on %s." % desc,
        committer=committer, update_changelog=update_changelog)


def add_test_dependency(tree, testname, package, minimum_version=None,
                        committer=None, subpath='', update_changelog=True):
    if not isinstance(package, str):
        raise TypeError(package)

    tests_control_path = os.path.join(
        tree.abspath(subpath), 'debian/tests/control')

    try:
        with Deb822Editor(path=tests_control_path) as updater:
            command_counter = 1
            for control in updater.paragraphs:
                try:
                    name = control["Tests"]
                except KeyError:
                    name = "command%d" % command_counter
                    command_counter += 1
                if name != testname:
                    continue
                if minimum_version:
                    control["Depends"] = ensure_minimum_version(
                        control["Depends"],
                        package, minimum_version)
                else:
                    control["Depends"] = ensure_some_version(
                        control["Depends"], package)
    except FormattingUnpreservable as e:
        note('Unable to edit %s in a way that preserves formatting.',
             e.path)
        return False
    if not updater.changed:
        return False

    if minimum_version:
        desc = "%s (>= %s)" % (package, minimum_version)
    else:
        desc = package

    note("Adding dependency to test %s: %s", testname, desc)
    return commit_debian_changes(
        tree, subpath,
        "Add missing dependency for test %s on %s." % (testname, desc),
        update_changelog=update_changelog)


def commit_debian_changes(tree, subpath, summary, committer=None,
                          update_changelog=True):
    with tree.lock_write():
        try:
            if update_changelog:
                add_changelog_entry(tree, subpath, [summary])
                debcommit(tree, committer=committer)
            else:
                tree.commit(message=summary, committer=committer)
        except PointlessCommit:
            return False
        else:
            return True


class FileSearcher(object):

    def search_files(self, path, regex=False):
        raise NotImplementedError(self.search_files)


class CliAptFileSearcher(FileSearcher):

    def search_files(self, path, regex=False):
        args = ['/usr/bin/apt-file', 'search', '-l']
        if regex:
            args.append('-x')
        else:
            args.append('-F')
        args.append(path)
        try:
            return iter(subprocess.check_output(args).decode().splitlines())
        except subprocess.CalledProcessError:
            return iter([])

    @classmethod
    def available(cls) -> bool:
        # TODO(jelmer): Also check whether database has been built?
        return os.path.exists('/usr/bin/apt-file')


class ContentsAptFileSearcher(FileSearcher):

    def __init__(self):
        self._db = {}

    def __setitem__(self, path, package):
        self._db[path] = package

    def search_files(self, path, regex=False):
        for p, pkg in sorted(self._db.items()):
            if regex:
                if re.match(path, p):
                    yield pkg
            else:
                if path == p:
                    yield pkg

    def load_file(self, f):
        for line in f:
            (path, rest) = line.rsplit(maxsplit=1)
            package = rest.split(b'/')[-1]
            decoded_path = '/' + path.decode('utf-8', 'surrogateescape')
            self[decoded_path] = package.decode('utf-8')

    def load_url(self, url):
        from urllib.request import urlopen, Request
        request = Request(url, headers={'User-Agent': 'Debian Janitor'})
        response = urlopen(request)
        if response.headers.get_content_type() == 'application/x-gzip':
            import gzip
            f = gzip.GzipFile(fileobj=response)
        elif response.headers.get_content_type() == 'text/plain':
            f = response
        else:
            raise Exception(
                'Unknown content type %r' %
                response.headers.get_content_type())
        self.load_file(f)


CONTENTS_URL = (
    'http://deb.debian.org/debian/dists/unstable/main/Contents-amd64.gz')


class GeneratedFileSearcher(FileSearcher):

    def __init__(self, db):
        self._db = db

    def search_files(self, path, regex=False):
        for p, pkg in sorted(self._db.items()):
            if regex:
                if re.match(path, p):
                    yield pkg
            else:
                if path == p:
                    yield pkg


# TODO(jelmer): read from a file
GENERATED_FILE_SEARCHER = GeneratedFileSearcher({
    '/etc/locale.gen': 'locales'})


_apt_file_searcher = None


def search_apt_file(path, regex=False):
    global _apt_file_searcher
    if _apt_file_searcher is None:
        # TODO(jelmer): Also check that apt-file uses unstable?
        if CliAptFileSearcher.available():
            _apt_file_searcher = CliAptFileSearcher()
        else:
            # TODO(jelmer): don't hardcode this URL
            # TODO(jelmer): cache file
            _apt_file_searcher = ContentsAptFileSearcher()
            _apt_file_searcher.load_url(CONTENTS_URL)
    yield from _apt_file_searcher.search_files(path, regex=regex)
    yield from GENERATED_FILE_SEARCHER.search_files(path, regex=regex)


def get_package_for_paths(paths, regex=False):
    candidates = set()
    for path in paths:
        candidates.update(search_apt_file(path, regex=regex))
        if candidates:
            break
    if len(candidates) == 0:
        warning('No packages found that contain %r', paths)
        return None
    if len(candidates) > 1:
        warning('More than 1 packages found that contain %r: %r',
                path, candidates)
        # Euhr. Pick the one with the shortest name?
        return sorted(candidates, key=len)[0]
    else:
        return candidates.pop()


def get_package_for_python_module(module, python_version):
    if python_version == 'python3':
        paths = [
            os.path.join(
                '/usr/lib/python3/dist-packages',
                module.replace('.', '/'),
                '__init__.py'),
            os.path.join(
                '/usr/lib/python3/dist-packages',
                module.replace('.', '/') + '.py'),
            os.path.join(
                '/usr/lib/python3\\.[0-9]+/lib-dynload',
                module.replace('.', '/') + '\\.cpython-.*\\.so'),
            ]
    elif python_version == 'python2':
        paths = [
            os.path.join(
                '/usr/lib/python2\\.[0-9]/dist-packages',
                module.replace('.', '/'),
                '__init__.py'),
            os.path.join(
                '/usr/lib/python2\\.[0-9]/dist-packages',
                module.replace('.', '/') + '.py'),
            os.path.join(
                '/usr/lib/python2.\\.[0-9]/lib-dynload',
                module.replace('.', '/') + '.so')]
    elif python_version == 'pypy':
        paths = [
            os.path.join(
                '/usr/lib/pypy/dist-packages',
                module.replace('.', '/'),
                '__init__.py'),
            os.path.join(
                '/usr/lib/pypy/dist-packages',
                module.replace('.', '/') + '.py'),
            os.path.join(
                '/usr/lib/pypy/dist-packages',
                module.replace('.', '/') + '\\.pypy-.*\\.so'),
            ]
    else:
        raise AssertionError(
            'unknown python version %r' % python_version)
    return get_package_for_paths(paths, regex=True)


def targeted_python_versions(tree):
    with tree.get_file('debian/control') as f:
        control = Deb822(f)
    build_depends = PkgRelation.parse_relations(
        control.get('Build-Depends', ''))
    all_build_deps = set()
    for or_deps in build_depends:
        all_build_deps.update(or_dep['name'] for or_dep in or_deps)
    targeted = set()
    if any(x.startswith('pypy') for x in all_build_deps):
        targeted.add('pypy')
    if any(x.startswith('python-') for x in all_build_deps):
        targeted.add('cpython2')
    if any(x.startswith('python3-') for x in all_build_deps):
        targeted.add('cpython3')
    return targeted


apt_cache = None


def package_exists(package):
    global apt_cache
    if apt_cache is None:
        import apt_pkg
        apt_cache = apt_pkg.Cache()
    for p in apt_cache.packages:
        if p.name == package:
            return True
    return False


def fix_missing_javascript_runtime(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    package = get_package_for_paths(
        ['/usr/bin/node', '/usr/bin/duk'],
        regex=False)
    if package is None:
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_python_distribution(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    targeted = targeted_python_versions(tree)
    default = not targeted

    pypy_pkg = get_package_for_paths(
        ['/usr/lib/pypy/dist-packages/%s-.*.egg-info' % error.distribution],
        regex=True)
    if pypy_pkg is None:
        pypy_pkg = 'pypy-%s' % error.distribution
        if not package_exists(pypy_pkg):
            pypy_pkg = None

    py2_pkg = get_package_for_paths(
        ['/usr/lib/python2\\.[0-9]/dist-packages/%s-.*.egg-info' %
         error.distribution], regex=True)
    if py2_pkg is None:
        py2_pkg = 'python-%s' % error.distribution
        if not package_exists(py2_pkg):
            py2_pkg = None

    py3_pkg = get_package_for_paths(
        ['/usr/lib/python3/dist-packages/%s-.*.egg-info' %
         error.distribution], regex=True)
    if py3_pkg is None:
        py3_pkg = 'python3-%s' % error.distribution
        if not package_exists(py3_pkg):
            py3_pkg = None

    extra_build_deps = []
    if error.python_version == 2:
        if 'pypy' in targeted:
            if not pypy_pkg:
                warning('no pypy package found for %s', error.module)
            else:
                extra_build_deps.append(pypy_pkg)
        if 'cpython2' in targeted or default:
            if not py2_pkg:
                warning('no python 2 package found for %s', error.module)
                return False
            extra_build_deps.append(py2_pkg)
    elif error.python_version == 3:
        if not py3_pkg:
            warning('no python 3 package found for %s', error.module)
            return False
        extra_build_deps.append(py3_pkg)
    else:
        if py3_pkg and ('cpython3' in targeted or default):
            extra_build_deps.append(py3_pkg)
        if py2_pkg and ('cpython2' in targeted or default):
            extra_build_deps.append(py2_pkg)
        if pypy_pkg and 'pypy' in targeted:
            extra_build_deps.append(pypy_pkg)

    if not extra_build_deps:
        return False

    for dep_pkg in extra_build_deps:
        assert dep_pkg is not None
        if not add_dependency(
                tree, context, dep_pkg, minimum_version=error.minimum_version,
                committer=committer, update_changelog=update_changelog,
                subpath=subpath):
            return False
    return True


def fix_missing_python_module(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    targeted = targeted_python_versions(tree)
    default = (not targeted)

    pypy_pkg = get_package_for_python_module(error.module, 'pypy')
    py2_pkg = get_package_for_python_module(error.module, 'python2')
    py3_pkg = get_package_for_python_module(error.module, 'python3')

    extra_build_deps = []
    if error.python_version == 2:
        if 'pypy' in targeted:
            if not pypy_pkg:
                warning('no pypy package found for %s', error.module)
            else:
                extra_build_deps.append(pypy_pkg)
        if 'cpython2' in targeted or default:
            if not py2_pkg:
                warning('no python 2 package found for %s', error.module)
                return False
            extra_build_deps.append(py2_pkg)
    elif error.python_version == 3:
        if not py3_pkg:
            warning('no python 3 package found for %s', error.module)
            return False
        extra_build_deps.append(py3_pkg)
    else:
        if py3_pkg and ('cpython3' in targeted or default):
            extra_build_deps.append(py3_pkg)
        if py2_pkg and ('cpython2' in targeted or default):
            extra_build_deps.append(py2_pkg)
        if pypy_pkg and 'pypy' in targeted:
            extra_build_deps.append(pypy_pkg)

    if not extra_build_deps:
        return False

    for dep_pkg in extra_build_deps:
        assert dep_pkg is not None
        if not add_dependency(
                tree, context, dep_pkg, error.minimum_version,
                committer=committer, update_changelog=update_changelog,
                subpath=subpath):
            return False
    return True


def fix_missing_go_package(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    package = get_package_for_paths(
        [os.path.join('/usr/share/gocode/src', error.package, '.*')],
        regex=True)
    if package is None:
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_c_header(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    package = get_package_for_paths(
        [os.path.join('/usr/include', error.header)], regex=False)
    if package is None:
        package = get_package_for_paths(
            [os.path.join('/usr/include', '.*', error.header)], regex=True)
    if package is None:
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_pkg_config(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    package = get_package_for_paths(
        [os.path.join('/usr/lib/pkgconfig', error.module + '.pc')])
    if package is None:
        package = get_package_for_paths(
            [os.path.join('/usr/lib', '.*', 'pkgconfig',
                          error.module + '.pc')],
            regex=True)
    if package is None:
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        minimum_version=error.minimum_version,
        update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_command(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    if os.path.isabs(error.command):
        paths = [error.command]
    else:
        paths = [
            os.path.join(dirname, error.command)
            for dirname in ['/usr/bin', '/bin']]
    package = get_package_for_paths(paths)
    if package is None:
        note('No packages found that contain %r', paths)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_file(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    package = get_package_for_paths([error.path])
    if package is None:
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_sprockets_file(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    if error.content_type == 'application/javascript':
        path = '/usr/share/.*/app/assets/javascripts/%s.js$' % error.name
    else:
        warning('unable to handle content type %s', error.content_type)
        return False
    package = get_package_for_paths([path], regex=True)
    if package is None:
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


DEFAULT_PERL_PATHS = ['/usr/share/perl5']


def fix_missing_perl_file(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):

    if (error.filename == 'Makefile.PL' and
            not tree.has_filename('Makefile.PL') and
            tree.has_filename('dist.ini')):
        # TODO(jelmer): add dist-zilla add-on to debhelper
        raise NotImplementedError

    if error.inc is None:
        if error.filename is None:
            filename = error.module.replace('::', '/') + '.pm'
            paths = [os.path.join(inc, filename)
                     for inc in DEFAULT_PERL_PATHS]
        elif not os.path.isabs(error.filename):
            return False
        else:
            paths = [error.filename]
    else:
        paths = [os.path.join(inc, error.filename) for inc in error.inc]
    package = get_package_for_paths(paths, regex=False)
    if package is None:
        if getattr(error, 'module', None):
            warning('no perl package found for %s (%r).',
                    error.module, error.filename)
        else:
            warning('perl file %s not found (paths searched for: %r).',
                    error.filename, paths)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def get_package_for_node_package(node_package):
    paths = [
        '/usr/share/nodejs/.*/node_modules/%s/package.json' % node_package,
        '/usr/lib/nodejs/%s/package.json' % node_package,
        '/usr/share/nodejs/%s/package.json' % node_package]
    return get_package_for_paths(paths, regex=True)


def fix_missing_node_module(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    package = get_package_for_node_package(error.module)
    if package is None:
        warning('no node package found for %s.',
                error.module)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_dh_addon(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    paths = [os.path.join('/usr/share/perl5', error.path)]
    package = get_package_for_paths(paths)
    if package is None:
        warning('no package for debhelper addon %s', error.name)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def retry_apt_failure(tree, error, context, committer=None,
                      update_changelog=True, subpath=''):
    return True


def fix_missing_php_class(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    path = '/usr/share/php/%s.php' % error.php_class.replace('\\', '/')
    package = get_package_for_paths([path])
    if package is None:
        warning('no package for PHP class %s', error.php_class)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_jdk_file(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    path = error.jdk_path + '.*/' % error.filename
    package = get_package_for_paths([path], regex=True)
    if package is None:
        warning('no package found for %s (JDK: %s) - regex %s',
                error.filename, error.jdk_path, path)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_vala_package(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    path = '/usr/share/vala-[0-9.]+/vapi/%s.vapi' % error.package
    package = get_package_for_paths([path], regex=True)
    if package is None:
        warning('no file found for package %s - regex %s',
                error.package, path)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_xml_entity(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    # Ideally we should be using the XML catalog for this, but hardcoding
    # a few URLs will do for now..
    URL_MAP = {
        'http://www.oasis-open.org/docbook/xml/':
            '/usr/share/xml/docbook/schema/dtd/'
    }
    for url, path in URL_MAP.items():
        if error.url.startswith(url):
            search_path = os.path.join(path, error.url[len(url):])
            break
    else:
        return False

    package = get_package_for_paths([search_path], regex=False)
    if package is None:
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_library(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    paths = [os.path.join('/usr/lib/lib%s.so$' % error.library),
             os.path.join('/usr/lib/.*/lib%s.so$' % error.library),
             os.path.join('/usr/lib/lib%s.a$' % error.library),
             os.path.join('/usr/lib/.*/lib%s.a$' % error.library)]
    package = get_package_for_paths(paths, regex=True)
    if package is None:
        warning('no package for library %s', error.library)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_ruby_gem(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    paths = [os.path.join(
        '/usr/share/rubygems-integration/all/'
        'specifications/%s-.*\\.gemspec' % error.gem)]
    package = get_package_for_paths(paths, regex=True)
    if package is None:
        warning('no package for gem %s', error.gem)
        return False
    return add_dependency(
        tree, context, package, minimum_version=error.version,
        committer=committer, update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_ruby_file(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    paths = [
        os.path.join('/usr/lib/ruby/vendor_ruby/%s.rb' % error.filename)]
    package = get_package_for_paths(paths)
    if package is not None:
        return add_dependency(
            tree, context, package, committer=committer,
            update_changelog=update_changelog, subpath=subpath)
    paths = [
        os.path.join(r'/usr/share/rubygems-integration/all/gems/([^/]+)/'
                     'lib/%s.rb' % error.filename)]
    package = get_package_for_paths(paths, regex=True)
    if package is not None:
        return add_dependency(
            tree, context, package, committer=committer,
            update_changelog=update_changelog, subpath=subpath)

    warning('no package for ruby file %s', error.filename)
    return False


def fix_missing_r_package(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    paths = [os.path.join('/usr/lib/R/site-library/.*/R/%s$' % error.package)]
    package = get_package_for_paths(paths, regex=True)
    if package is None:
        warning('no package for R package %s', error.package)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        minimum_version=error.minimum_version,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_java_class(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    # Unfortunately this only finds classes in jars installed on the host
    # system :(
    output = subprocess.check_output(
        ["java-propose-classpath", "-c" + error.classname])
    classpath = [
        p for p in output.decode().strip(":").strip().split(':') if p]
    if not classpath:
        warning('unable to find classpath for %s', error.classname)
        return False
    note('Classpath for %s: %r', error.classname, classpath)
    package = get_package_for_paths(classpath)
    if package is None:
        warning('no package for files in %r', classpath)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def enable_dh_autoreconf(tree, context, committer, update_changelog=True,
                         subpath=''):
    # Debhelper >= 10 depends on dh-autoreconf and enables autoreconf by
    # default.
    debhelper_compat_version = get_debhelper_compat_level(tree.abspath('.'))
    if debhelper_compat_version is not None and debhelper_compat_version < 10:
        def add_with_autoreconf(line, target):
            if target != b'%':
                return line
            if not line.startswith(b'dh '):
                return line
            return dh_invoke_add_with(line, b'autoreconf')

        if update_rules(command_line_cb=add_with_autoreconf):
            return add_dependency(
                tree, context, 'dh-autoreconf', committer=committer,
                update_changelog=update_changelog, subpath=subpath)

    return False


def fix_missing_configure(tree, error, context, committer=None,
                          update_changelog=True, subpath=''):
    if (not tree.has_filename('configure.ac') and
            not tree.has_filename('configure.in')):
        return False

    return enable_dh_autoreconf(
        tree, context, committer=committer, update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_automake_input(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    # TODO(jelmer): If it's ./NEWS, ./AUTHORS or ./README that's missing, then
    # try to set 'export AUTOMAKE = automake --foreign' in debian/rules.
    # https://salsa.debian.org/jelmer/debian-janitor/issues/88
    return enable_dh_autoreconf(
        tree, context, committer=committer, update_changelog=update_changelog,
        subpath=subpath)


def fix_missing_maven_artifacts(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    artifact = error.artifacts[0]
    try:
        (group_id, artifact_id, kind, version) = artifact.split(':')
    except ValueError:
        (group_id, artifact_id, version) = artifact.split(':')
        kind = 'jar'
    paths = [os.path.join(
        '/usr/share/maven-repo', group_id.replace('.', '/'),
        artifact_id, version, '%s-%s.%s' % (artifact_id, version, kind))]
    package = get_package_for_paths(paths)
    if package is None:
        warning('no package for artifact %s', artifact)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def install_gnome_common(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    return add_dependency(
        tree, context, 'gnome-common', committer=committer,
        update_changelog=update_changelog, subpath=subpath)


def fix_missing_config_status_input(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    autogen_path = 'autogen.sh'
    rules_path = 'debian/rules'
    if subpath not in ('.', ''):
        autogen_path = os.path.join(subpath, autogen_path)
        rules_path = os.path.join(subpath, rules_path)
    if not tree.has_filename(autogen_path):
        return False

    def add_autogen(mf):
        rule = any(mf.iter_rules(b'override_dh_autoreconf'))
        if rule:
            return
        rule = mf.add_rule(b'override_dh_autoreconf')
        rule.append_command(b'dh_autoreconf ./autogen.sh')

    if not update_rules(makefile_cb=add_autogen, path=rules_path):
        return False

    if update_changelog:
        commit_debian_changes(
            tree, subpath, 'Run autogen.sh during build.', committer=committer,
            update_changelog=update_changelog)

    return True


def _find_aclocal_fun(macro):
    # TODO(jelmer): Use the API for codesearch.debian.net instead?
    defun_prefix = b'AC_DEFUNE([%s],' % macro.encode('ascii')
    for entry in os.scandir('/usr/share/aclocal'):
        if not entry.is_file():
            continue
        with open(entry.path, 'rb') as f:
            for line in f:
                if line.startswith(defun_prefix):
                    return entry.path
    raise KeyError


def run_pgbuildext_updatecontrol(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    note("Running 'pg_buildext updatecontrol'")
    pg_buildext_updatecontrol(tree.abspath(subpath))
    return commit_debian_changes(
        tree, subpath, "Run 'pgbuildext updatecontrol'.",
        committer=committer, update_changelog=False)


def fix_missing_autoconf_macro(
        tree, error, context, committer=None, update_changelog=True,
        subpath=''):
    try:
        path = _find_aclocal_fun(error.macro)
    except KeyError:
        note('No local m4 file found defining %s', error.macro)
        return False
    package = get_package_for_paths([path])
    if package is None:
        warning('no package for macro file %s', path)
        return False
    return add_dependency(
        tree, context, package, committer=committer,
        update_changelog=update_changelog, subpath=subpath)


FIXERS = [
    (MissingPythonModule, fix_missing_python_module),
    (MissingPythonDistribution, fix_missing_python_distribution),
    (MissingCHeader, fix_missing_c_header),
    (MissingPkgConfig, fix_missing_pkg_config),
    (MissingCommand, fix_missing_command),
    (MissingFile, fix_missing_file),
    (MissingSprocketsFile, fix_missing_sprockets_file),
    (MissingGoPackage, fix_missing_go_package),
    (MissingPerlFile, fix_missing_perl_file),
    (MissingPerlModule, fix_missing_perl_file),
    (MissingXmlEntity, fix_missing_xml_entity),
    (MissingNodeModule, fix_missing_node_module),
    (MissingRubyGem, fix_missing_ruby_gem),
    (MissingRPackage, fix_missing_r_package),
    (MissingLibrary, fix_missing_library),
    (MissingJavaClass, fix_missing_java_class),
    (MissingConfigure, fix_missing_configure),
    (MissingAutomakeInput, fix_missing_automake_input),
    (DhAddonLoadFailure, fix_missing_dh_addon),
    (MissingPhpClass, fix_missing_php_class),
    (AptFetchFailure, retry_apt_failure),
    (MissingMavenArtifacts, fix_missing_maven_artifacts),
    (GnomeCommonMissing, install_gnome_common),
    (MissingConfigStatusInput, fix_missing_config_status_input),
    (MissingJDKFile, fix_missing_jdk_file),
    (MissingRubyFile, fix_missing_ruby_file),
    (MissingJavaScriptRuntime, fix_missing_javascript_runtime),
    (MissingAutoconfMacro, fix_missing_autoconf_macro),
    (NeedPgBuildExtUpdateControl, run_pgbuildext_updatecontrol),
    (MissingValaPackage, fix_missing_vala_package),
]


def resolve_error(tree, error, context, committer=None, subpath=''):
    for error_cls, fixer in FIXERS:
        if isinstance(error, error_cls):
            note('Attempting to use fixer %r to address %r',
                 fixer, error)
            try:
                return fixer(tree, error, context, committer, subpath)
            except GeneratedFile:
                warning('Control file is generated, unable to edit.')
                return False
    warning('No fixer found for %r', error)
    return False


def build_incrementally(
        local_tree, suffix, build_suite, output_directory, build_command,
        build_changelog_entry='Build for debian-janitor apt repository.',
        committer=None, max_iterations=DEFAULT_MAX_ITERATIONS,
        subpath='', source_date_epoch=None, update_changelog=True):
    fixed_errors = []
    while True:
        try:
            return attempt_build(
                local_tree, suffix, build_suite, output_directory,
                build_command, build_changelog_entry, subpath=subpath,
                source_date_epoch=source_date_epoch)
        except SbuildFailure as e:
            if e.error is None:
                warning('Build failed with unidentified error. Giving up.')
                raise
            if (e.error, e.context) in fixed_errors:
                warning('Error was still not fixed on second try. Giving up.')
                raise
            if max_iterations is not None \
                    and len(fixed_errors) > max_iterations:
                warning('Last fix did not address the issue. Giving up.')
                raise
            reset_tree(local_tree, subpath=subpath)
            try:
                if not resolve_error(
                        local_tree, e.error, e.context, committer=committer,
                        subpath=subpath):
                    warning('Failed to resolve error %r. Giving up.', e.error)
                    raise
            except CircularDependency:
                warning('Unable to fix %r; it would introduce a circular '
                        'dependency.', e.error)
                raise e
            fixed_errors.append((e.error, e.context))
            if os.path.exists(os.path.join(output_directory, 'build.log')):
                i = 1
                while os.path.exists(
                        os.path.join(output_directory, 'build.log.%d' % i)):
                    i += 1
                os.rename(os.path.join(output_directory, 'build.log'),
                          os.path.join(output_directory, 'build.log.%d' % i))


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser('janitor.fix_build')
    parser.add_argument('--suffix', type=str,
                        help="Suffix to use for test builds.",
                        default='fixbuild1')
    parser.add_argument('--suite', type=str,
                        help="Suite to target.",
                        default='unstable')
    parser.add_argument('--output-directory', type=str,
                        help="Output directory.", default=None)
    parser.add_argument('--committer', type=str,
                        help='Committer string (name and email)',
                        default=None)
    parser.add_argument(
        '--build-command', type=str,
        help='Build command',
        default=(DEFAULT_BUILDER + ' -A -s -v -d$DISTRIBUTION'))
    parser.add_argument(
        '--no-update-changelog', action="store_false", default=None,
        dest="update_changelog", help="do not update the changelog")
    parser.add_argument(
        '--update-changelog', action="store_true", dest="update_changelog",
        help="force updating of the changelog", default=None)

    args = parser.parse_args()
    from breezy.workingtree import WorkingTree
    tree = WorkingTree.open('.')
    build_incrementally(
        tree, args.suffix, args.suite, args.output_directory,
        args.build_command, committer=args.committer,
        update_changelog=args.update_changelog)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
