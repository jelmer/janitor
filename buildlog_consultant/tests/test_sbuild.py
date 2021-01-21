#!/usr/bin/python
# Copyright (C) 2019-2021 Jelmer Vernooij <jelmer@jelmer.uk>
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

from buildlog_consultant.sbuild import (
    AptFetchFailure,
    AptMissingReleaseFile,
    AutopkgtestTestbedFailure,
    AutopkgtestDepsUnsatisfiable,
    AutopkgtestDepChrootDisappeared,
    AutopkgtestTimedOut,
    AutopkgtestStderrFailure,
    CMakeFilesMissing,
    find_apt_get_failure,
    find_autopkgtest_failure_description,
    find_build_failure_description,
    CcacheError,
    DebhelperPatternNotFound,
    DuplicateDHCompatLevel,
    DhLinkDestinationIsDirectory,
    InconsistentSourceFormat,
    MissingConfigure,
    MissingJavaScriptRuntime,
    MissingJVM,
    MissingConfigStatusInput,
    MissingCHeader,
    MissingDHCompatLevel,
    MissingJDKFile,
    MissingPythonModule,
    MissingPythonDistribution,
    MissingGoPackage,
    MissingFile,
    MissingMavenArtifacts,
    MissingNodeModule,
    MissingCommand,
    MissingPkgConfig,
    MissingPerlFile,
    MissingPerlModule,
    MissingPhpClass,
    MissingRubyGem,
    MissingValaPackage,
    MissingXmlEntity,
    MissingLibrary,
    MissingJavaClass,
    MissingRPackage,
    MissingAutoconfMacro,
    MissingSprocketsFile,
    MissingAutomakeInput,
    NeedPgBuildExtUpdateControl,
    DhMissingUninstalled,
    DhUntilUnsupported,
    DhAddonLoadFailure,
    NoSpaceOnDevice,
    DhWithOrderIncorrect,
    FailedGoTest,
    UpstartFilePresent,
    DirectoryNonExistant,
    parse_brz_error,
    )
import unittest


class FindBuildFailureDescriptionTests(unittest.TestCase):

    def run_test(self, lines, lineno, err=None):
        (offset, actual_line, actual_err) = find_build_failure_description(
            lines)
        if lineno is not None:
            self.assertEqual(actual_line, lines[lineno-1])
            self.assertEqual(lineno, offset)
        else:
            self.assertIs(actual_line, None)
            self.assertIs(offset, None)
        if err:
            self.assertEqual(actual_err, err)
        else:
            self.assertIs(None, actual_err)

    def test_make_missing_rule(self):
        self.run_test([
            'make[1]: *** No rule to make target \'nno.autopgen.bin\', '
            'needed by \'dan-nno.autopgen.bin\'.  Stop.'],
            1)
        self.run_test([
            'make[1]: *** No rule to make target \'/usr/share/blah/blah\', '
            'needed by \'dan-nno.autopgen.bin\'.  Stop.'],
            1, MissingFile('/usr/share/blah/blah'))
        self.run_test([
            'debian/rules:4: /usr/share/openstack-pkg-tools/pkgos.make: '
            'No such file or directory'], 1,
            MissingFile('/usr/share/openstack-pkg-tools/pkgos.make'))

    def test_ioerror(self):
        self.run_test([
            'E   IOError: [Errno 2] No such file or directory: '
            '\'/usr/lib/python2.7/poly1305/rfc7539.txt\''], 1,
            MissingFile('/usr/lib/python2.7/poly1305/rfc7539.txt'))

    def test_upstart_file_present(self):
        self.run_test([
            'dh_installinit: upstart jobs are no longer supported!  '
            'Please remove debian/sddm.upstart and check if you '
            'need to add a conffile removal'], 1,
            UpstartFilePresent('debian/sddm.upstart'))

    def test_missing_javascript_runtime(self):
        self.run_test([
            'ExecJS::RuntimeUnavailable: '
            'Could not find a JavaScript runtime. '
            'See https://github.com/rails/execjs for a list '
            'of available runtimes.'], 1,
            MissingJavaScriptRuntime())

    def test_directory_missing(self):
        self.run_test([
            'debian/components/build: 19: cd: can\'t cd to rollup-plugin',
            ], 1, DirectoryNonExistant('rollup-plugin'))

    def test_missing_sprockets_file(self):
        self.run_test([
            'Sprockets::FileNotFound: couldn\'t find file '
            '\'activestorage\' with type \'application/javascript\''], 1,
            MissingSprocketsFile('activestorage', 'application/javascript'))

    def test_gxx_missing_file(self):
        self.run_test([
            'g++: error: /usr/lib/x86_64-linux-gnu/libGL.so: '
            'No such file or directory'], 1,
            MissingFile('/usr/lib/x86_64-linux-gnu/libGL.so'))

    def test_build_xml_missing_file(self):
        self.run_test([
            '/<<PKGBUILDDIR>>/build.xml:59: '
            '/<<PKGBUILDDIR>>/lib does not exist.'], 1,
            None)

    def test_dh_missing_addon(self):
        self.run_test([
           '   dh_auto_clean -O--buildsystem=pybuild',
           'E: Please add appropriate interpreter package to Build-Depends, '
           'see pybuild(1) for details.this: $VAR1 = bless( {',
           "     'py3vers' => '3.8',",
           "     'py3def' => '3.8',",
           "     'pyvers' => '',",
           "     'parallel' => '2',",
           "     'cwd' => '/<<PKGBUILDDIR>>',",
           "     'sourcedir' => '.',",
           "     'builddir' => undef,",
           "     'pypydef' => '',",
           "     'pydef' => ''",
           "   }, 'Debian::Debhelper::Buildsystem::pybuild' );",
           "deps: $VAR1 = [];"], 2,
           DhAddonLoadFailure(
               'pybuild', 'Debian/Debhelper/Buildsystem/pybuild.pm'))

    def test_libtoolize_missing_file(self):
        self.run_test([
            "libtoolize:   error: '/usr/share/aclocal/ltdl.m4' "
            "does not exist."], 1, MissingFile('/usr/share/aclocal/ltdl.m4'))

    def test_ruby_missing_file(self):
        self.run_test([
            "Error: Error: ENOENT: no such file or directory, "
            "open '/usr/lib/nodejs/requirejs/text.js'"], 1,
            MissingFile('/usr/lib/nodejs/requirejs/text.js'))

    def test_python_missing_file(self):
        self.run_test([
            "python3.7: can't open file '/usr/bin/blah.py': "
            "[Errno 2] No such file or directory"], 1,
            MissingFile('/usr/bin/blah.py'))
        self.run_test([
            "python3.7: can't open file 'setup.py': "
            "[Errno 2] No such file or directory"], 1)
        self.run_test([
            "E           FileNotFoundError: [Errno 2] "
            "No such file or directory: "
            "'/usr/share/firmware-microbit-micropython/firmware.hex'"],
            1, MissingFile(
                '/usr/share/firmware-microbit-micropython/firmware.hex'))

    def test_interpreter_missing(self):
        self.run_test([
            '/bin/bash: /usr/bin/rst2man: /usr/bin/python: '
            'bad interpreter: No such file or directory'], 1,
            MissingFile('/usr/bin/python'))
        self.run_test([
            'env: ‘/<<PKGBUILDDIR>>/socket-activate’: '
            'No such file or directory'], 1, None)

    def test_webpack_missing(self):
        self.run_test([
            "ERROR in Entry module not found: "
            "Error: Can't resolve 'index.js' in '/<<PKGBUILDDIR>>'"], 1,
            None)

    def test_installdocs_missing(self):
        self.run_test([
            'dh_installdocs: Cannot find (any matches for) "README.txt" '
            '(tried in ., debian/tmp)'],
            1, DebhelperPatternNotFound(
                'README.txt', 'installdocs', ['.', 'debian/tmp']))

    def test_cmake_missing_file(self):
        self.run_test("""\
CMake Error at /usr/lib/x86_64-/cmake/Qt5Gui/Qt5GuiConfig.cmake:27 (message):
  The imported target "Qt5::Gui" references the file

     "/usr/lib/x86_64-linux-gnu/libEGL.so"

  but this file does not exist.  Possible reasons include:

  * The file was deleted, renamed, or moved to another location.

  * An install or uninstall procedure did not complete successfully.

  * The installation package was faulty and contained

     "/usr/lib/x86_64-linux-gnu/cmake/Qt5Gui/Qt5GuiConfigExtras.cmake"

  but not all the files it references.

Call Stack (most recent call first):
  /usr/lib/x86_64-linux-gnu/QtGui/Qt5Gui.cmake:63 (_qt5_Gui_check_file_exists)
  /usr/lib/x86_64-linux-gnu/QtGui/Qt5Gui.cmake:85 (_qt5gui_find_extra_libs)
  /usr/lib/x86_64-linux-gnu/QtGui/Qt5Gui.cmake:186 (include)
  /usr/lib/x86_64-linux-gnu/QtWidgets/Qt5Widgets.cmake:101 (find_package)
  /usr/lib/x86_64-linux-gnu/Qt/Qt5Config.cmake:28 (find_package)
  CMakeLists.txt:34 (find_package)
dh_auto_configure: cd obj-x86_64-linux-gnu && cmake with args
""".splitlines(True), 16, MissingFile('/usr/lib/x86_64-linux-gnu/libEGL.so'))

    def test_meson_missing_git(self):
        self.run_test([
            'meson.build:13:0: ERROR: Git program not found.'], 1,
            MissingCommand('git'))

    def test_need_pgbuildext(self):
        self.run_test([
            "Error: debian/control needs updating from debian/control.in. "
            "Run 'pg_buildext updatecontrol'."], 1,
            NeedPgBuildExtUpdateControl('debian/control', 'debian/control.in'))

    def test_cmake_missing_command(self):
        self.run_test([
            '  Could NOT find Git (missing: GIT_EXECUTABLE)',
            'dh_auto_configure: cd obj-x86_64-linux-gnu && cmake with args'],
            1, MissingCommand('git'))

    def test_cmake_missing_cmake_files(self):
        self.run_test("""\
  Could not find a package configuration file provided by "sensor_msgs" with
  any of the following names:

    sensor_msgsConfig.cmake
    sensor_msgs-config.cmake

  Add the installation prefix of "sensor_msgs" to CMAKE_PREFIX_PATH or set
  "sensor_msgs_DIR" to a directory containing one of the above files.  If
  "sensor_msgs" provides a separate development package or SDK, be sure it
  has been installed.
dh_auto_configure: cd obj-x86_64-linux-gnu && cmake with args
""".splitlines(True), 1, CMakeFilesMissing([
            'sensor_msgsConfig.cmake', 'sensor_msgs-config.cmake']))

    def test_dh_compat_dupe(self):
        self.run_test([
            'dh_autoreconf: debhelper compat level specified both in '
            'debian/compat and via build-dependency on debhelper-compat'], 1,
            DuplicateDHCompatLevel('dh_autoreconf'))

    def test_dh_compat_missing(self):
        self.run_test([
            'dh_clean: Please specify the compatibility level in '
            'debian/compat'], 1, MissingDHCompatLevel('dh_clean'))

    def test_dh_udeb_shared_library(self):
        self.run_test([
            'dh_makeshlibs: The udeb libepoxy0-udeb (>= 1.3) does not contain'
            ' any shared libraries but --add-udeb=libepoxy0-udeb (>= 1.3) '
            'was passed!?'], 1)

    def test_dh_systemd(self):
        self.run_test([
            'dh: unable to load addon systemd: dh: The systemd-sequence is '
            'no longer provided in compat >= 11, please rely on '
            'dh_installsystemd instead'], 1)

    def test_dh_before(self):
        self.run_test([
            'dh: The --before option is not supported any longer (#932537). '
            'Use override targets instead.'], 1)

    def test_distutils_missing(self):
        self.run_test([
            'distutils.errors.DistutilsError: Could not find suitable '
            'distribution for Requirement.parse(\'pytest-runner\')'],
            1, MissingPythonDistribution('pytest-runner', None))
        self.run_test([
            "distutils.errors.DistutilsError: Could not find suitable "
            "distribution for Requirement.parse('certifi>=2019.3.9')"],
            1, MissingPythonDistribution('certifi', None, '2019.3.9'))
        self.run_test([
            'distutils.errors.DistutilsError: Could not find suitable '
            'distribution for Requirement.parse(\'cffi; '
            'platform_python_implementation == "CPython"\')'], 1,
            MissingPythonDistribution('cffi', None))
        self.run_test([
            'error: Could not find suitable distribution for '
            'Requirement.parse(\'gitlab\')'], 1,
            MissingPythonDistribution('gitlab', None))
        self.run_test([
            'pkg_resources.DistributionNotFound: The \'configparser>=3.5\' '
            'distribution was not found and is required by importlib-metadata'
            ], 1, MissingPythonDistribution('configparser', None, '3.5'))

    def test_pytest_import(self):
        self.run_test([
            'E   ImportError: cannot import name cmod'], 1,
            MissingPythonModule('cmod'))
        self.run_test([
            'E   ImportError: No module named mock'], 1,
            MissingPythonModule('mock'))
        self.run_test([
            'pluggy.manager.PluginValidationError: '
            'Plugin \'xdist.looponfail\' could not be loaded: '
            '(pytest 3.10.1 (/usr/lib/python2.7/dist-packages), '
            'Requirement.parse(\'pytest>=4.4.0\'))!'], 1,
            MissingPythonModule('pytest', 2, '4.4.0'))
        self.run_test(
                ['ImportError: Error importing plugin '
                 '"tests.plugins.mock_libudev": No module named mock'], 1,
                MissingPythonModule('mock'))

    def test_sed(self):
        self.run_test(
            ['sed: can\'t read /etc/locale.gen: No such file or directory'], 1,
            MissingFile('/etc/locale.gen'))

    def test_python2_import(self):
        self.run_test(
                ['ImportError: No module named pytz'], 1,
                MissingPythonModule('pytz'))
        self.run_test(
                ['ImportError: cannot import name SubfieldBase'], 1,
                None)

    def test_python3_import(self):
        self.run_test([
            'ModuleNotFoundError: No module named \'django_crispy_forms\''], 1,
            MissingPythonModule('django_crispy_forms', 3))
        self.run_test([
            'ModuleNotFoundError: No module named \'distro\''], 1,
            MissingPythonModule('distro', 3))
        self.run_test([
            'E   ModuleNotFoundError: No module named \'twisted\''], 1,
            MissingPythonModule('twisted', 3))
        self.run_test([
            'E   ImportError: cannot import name \'async_poller\' '
            'from \'msrest.polling\' '
            '(/usr/lib/python3/dist-packages/msrest/polling/__init__.py)'], 1,
            MissingPythonModule('msrest.polling'))
        self.run_test([
            '/usr/bin/python3: No module named sphinx'], 1,
            MissingPythonModule('sphinx', 3))
        self.run_test([
            'Could not import extension sphinx.ext.pngmath (exception: '
            'No module named pngmath)'], 1, MissingPythonModule('pngmath'))

    def test_sphinx(self):
        self.run_test([
            'There is a syntax error in your configuration file: '
            'Unknown syntax: Constant'], 1, None)

    def test_go_missing(self):
        self.run_test([
            'src/github.com/vuls/config/config.go:30:2: cannot find package '
            '"golang.org/x/xerrors" in any of:'], 1,
            MissingGoPackage('golang.org/x/xerrors'))

    def test_c_header_missing(self):
        self.run_test([
            'cdhit-common.h:39:9: fatal error: zlib.h: No such file '
            'or directory'], 1,
            MissingCHeader('zlib.h'))
        self.run_test([
            '/<<PKGBUILDDIR>>/Kernel/Operation_Vector.cpp:15:10: '
            'fatal error: petscvec.h: No such file or directory'], 1,
            MissingCHeader('petscvec.h'))
        self.run_test([
            'src/bubble.h:27:10: fatal error: DBlurEffectWidget: '
            'No such file or directory'], 1,
            MissingCHeader('DBlurEffectWidget'))

    def test_missing_jdk_file(self):
        self.run_test([
            '> Could not find tools.jar. Please check that '
            '/usr/lib/jvm/java-8-openjdk-amd64 contains a '
            'valid JDK installation.',
            ], 1, MissingJDKFile(
                '/usr/lib/jvm/java-8-openjdk-amd64', 'tools.jar'))

    def test_node_module_missing(self):
        self.run_test([
            'Error: Cannot find module \'tape\''], 1,
            MissingNodeModule('tape'))
        self.run_test([
            '✖ [31mERROR:[39m Cannot find module \'/<<PKGBUILDDIR>>/test\'',
            ], 1, None)

    def test_command_missing(self):
        self.run_test([
            './ylwrap: line 176: yacc: command not found'], 1,
            MissingCommand('yacc'))
        self.run_test([
            '/bin/sh: 1: cmake: not found'], 1,
            MissingCommand('cmake'))
        self.run_test([
            'sh: 1: git: not found'], 1,
            MissingCommand('git'))
        self.run_test([
            '/usr/bin/env: ‘python3’: No such file or directory'], 1,
            MissingCommand('python3'))
        self.run_test([
            'make[1]: docker: Command not found'], 1,
            MissingCommand('docker'))
        self.run_test([
            'make[1]: git: Command not found'], 1,
            MissingCommand('git'))
        self.run_test(['make[1]: ./docker: Command not found'], 1, None)
        self.run_test([
            'make: dh_elpa: Command not found'], 1, MissingCommand('dh_elpa'))
        self.run_test([
            '/bin/bash: valac: command not found'], 1,
            MissingCommand('valac'))
        self.run_test([
            'Can\'t exec "cmake": No such file or directory at '
            '/usr/share/perl5/Debian/Debhelper/Dh_Lib.pm line 484.'], 1,
            MissingCommand('cmake'))
        self.run_test([
            'Invalid gemspec in [unicorn.gemspec]: '
            'No such file or directory - git'],
            1, MissingCommand('git'))
        self.run_test([
            'dbus-run-session: failed to exec \'xvfb-run\': '
            'No such file or directory'], 1,
            MissingCommand('xvfb-run'))
        self.run_test(
            ['/bin/sh: 1: ./configure: not found'], 1,
            MissingConfigure())
        self.run_test(
            ['xvfb-run: error: xauth command not found'], 1,
            MissingCommand('xauth'))
        self.run_test(
            ['meson.build:39:2: ERROR: Program(s) [\'wrc\'] '
             'not found or not executable'], 1,
            MissingCommand('wrc'))
        self.run_test(
            ['/tmp/autopkgtest.FnbV06/build.18W/src/debian/tests/'
             'blas-testsuite: 7: dpkg-architecture: not found'],
            1, MissingCommand('dpkg-architecture'))

    def test_ts_error(self):
        self.run_test([
            'blah/tokenizer.ts(175,21): error TS2532: '
            'Object is possibly \'undefined\'.'], 1, None)

    def test_nim_error(self):
        self.run_test([
            '/<<PKGBUILDDIR>>/msgpack4nim.nim(470, 6) '
            'Error: usage of \'isNil\' is a user-defined error'], 1, None)

    def test_scala_error(self):
        self.run_test([
            'core/src/main/scala/org/json4s/JsonFormat.scala:131: '
            'error: No JSON deserializer found for type List[T]. '
            'Try to implement an implicit Reader or JsonFormat for this type.'
            ], 1, None)

    def test_vala_error(self):
        self.run_test([
            '../src/Backend/FeedServer.vala:60.98-60.148: error: '
            'The name `COLLECTION_CREATE_NONE\' does not exist in '
            'the context of `Secret.CollectionCreateFlags\''], 1,
            None)
        self.run_test([
            'error: Package `glib-2.0\' not found in specified Vala '
            'API directories or GObject-Introspection GIR directories'],
            1, MissingValaPackage('glib-2.0'))

    def test_pkg_config_missing(self):
        self.run_test([
            'configure: error: Package requirements '
            '(apertium-3.2 >= 3.2.0) were not met:'],
            1, MissingPkgConfig('apertium-3.2', '3.2.0'))
        self.run_test([
            'meson.build:10:0: ERROR: Dependency "gssdp-1.2" not '
            'found, tried pkgconfig'], 1, MissingPkgConfig('gssdp-1.2'))
        self.run_test([
            'src/plugins/sysprof/meson.build:3:0: '
            'ERROR: Dependency "sysprof-3" not found, tried pkgconfig'],
            1, MissingPkgConfig('sysprof-3'))
        self.run_test([
            'meson.build:84:0: ERROR: Invalid version of dependency, '
            'need \'libpeas-1.0\' [\'>= 1.24.0\'] found \'1.22.0\'.'], 1,
            MissingPkgConfig('libpeas-1.0', '1.24.0'))
        self.run_test([
            'No package \'tepl-3\' found'], 1, MissingPkgConfig('tepl-3'))
        self.run_test([
            'Requested \'vte-2.91 >= 0.59.0\' but version of vte is 0.58.2'],
            1, MissingPkgConfig('vte-2.91', '0.59.0'))
        self.run_test([
            'configure: error: x86_64-linux-gnu-pkg-config sdl2 couldn\'t '
            'be found'], 1, MissingPkgConfig('sdl2'))
        self.run_test([
            'configure: error: No package \'libcrypto\' found'], 1,
            MissingPkgConfig('libcrypto'))
        self.run_test([
            "-- Checking for module 'gtk+-3.0'",
            "--   Package 'gtk+-3.0', required by 'virtual:world', not found"],
            2, MissingPkgConfig('gtk+-3.0'))
        self.run_test([
            'configure: error: libfilezilla not found: Package dependency '
            'requirement \'libfilezilla >= 0.17.1\' could not be satisfied.'],
            1, MissingPkgConfig('libfilezilla', '0.17.1'))

    def test_dh_with_order(self):
        self.run_test([
            'dh: Unknown sequence --with '
            '(options should not come before the sequence)'], 1,
            DhWithOrderIncorrect())

    def test_no_disk_space(self):
        self.run_test([
            '/usr/bin/install: error writing \''
            '/<<PKGBUILDDIR>>/debian/tmp/usr/lib/gcc/'
            'x86_64-linux-gnu/8/cc1objplus\': No space left on device'], 1,
            NoSpaceOnDevice())

    def test_segmentation_fault(self):
        self.run_test([
            '/bin/bash: line 3:  7392 Segmentation fault      '
            'itstool -m "${mo}" ${d}/C/index.docbook ${d}/C/legal.xml'], 1)

    def test_missing_perl_plugin(self):
        self.run_test([
            'Required plugin bundle Dist::Zilla::PluginBundle::Git isn\'t '
            'installed.'], 1,
            MissingPerlModule(None, 'Dist::Zilla::PluginBundle::Git', None))
        self.run_test([
            'Required plugin Dist::Zilla::Plugin::PPPort isn\'t installed.'],
            1, MissingPerlModule(
                filename=None, module='Dist::Zilla::Plugin::PPPort'))

    def test_perl_expand(self):
        self.run_test([
            ">(error): Could not expand [ 'Dist::Inkt::Profile::TOBYINK'"],
            1, MissingPerlModule(None, module='Dist::Inkt::Profile::TOBYINK'))

    def test_missing_perl_module(self):
        self.run_test([
            'Converting tags.ledger... Can\'t locate String/Interpolate.pm in '
            '@INC (you may need to install the String::Interpolate module) '
            '(@INC contains: /etc/perl /usr/local/lib/x86_64-linux-gnu/perl/'
            '5.28.1 /usr/local/share/perl/5.28.1 /usr/lib/x86_64-linux-gnu/'
            'perl5/5.28 /usr/share/perl5 /usr/lib/x86_64-linux-gnu/perl/5.28 '
            '/usr/share/perl/5.28 /usr/local/lib/site_perl '
            '/usr/lib/x86_64-linux-gnu/perl-base) at '
            '../bin/ledger2beancount line 23.'], 1,
            MissingPerlModule('String/Interpolate.pm', 'String::Interpolate', [
                '/etc/perl', '/usr/local/lib/x86_64-linux-gnu/perl/5.28.1',
                '/usr/local/share/perl/5.28.1',
                '/usr/lib/x86_64-linux-gnu/perl5/5.28',
                '/usr/share/perl5', '/usr/lib/x86_64-linux-gnu/perl/5.28',
                '/usr/share/perl/5.28', '/usr/local/lib/site_perl',
                '/usr/lib/x86_64-linux-gnu/perl-base']))
        self.run_test([
            'Can\'t locate Test/Needs.pm in @INC '
            '(you may need to install the Test::Needs module) '
            '(@INC contains: t/lib /<<PKGBUILDDIR>>/blib/lib '
            '/<<PKGBUILDDIR>>/blib/arch /etc/perl '
            '/usr/local/lib/x86_64-linux-gnu/perl/5.30.0 '
            '/usr/local/share/perl/5.30.0 /usr/lib/x86_64-linux-gnu/perl5/5.30'
            ' /usr/share/perl5 /usr/lib/x86_64-linux-gnu/perl/5.30 '
            '/usr/share/perl/5.30 /usr/local/lib/site_perl '
            '/usr/lib/x86_64-linux-gnu/perl-base .) at '
            't/anon-basic.t line 7.'], 1,
            MissingPerlModule('Test/Needs.pm', 'Test::Needs', [
                't/lib', '/<<PKGBUILDDIR>>/blib/lib',
                '/<<PKGBUILDDIR>>/blib/arch', '/etc/perl',
                '/usr/local/lib/x86_64-linux-gnu/perl/5.30.0',
                '/usr/local/share/perl/5.30.0',
                '/usr/lib/x86_64-linux-gnu/perl5/5.30',
                '/usr/share/perl5', '/usr/lib/x86_64-linux-gnu/perl/5.30',
                '/usr/share/perl/5.30', '/usr/local/lib/site_perl',
                '/usr/lib/x86_64-linux-gnu/perl-base', '.']))

    def test_missing_perl_file(self):
        self.run_test([
            'Can\'t locate debian/perldl.conf in @INC (@INC contains: '
            '/<<PKGBUILDDIR>>/inc /etc/perl /usr/local/lib/x86_64-linux-gnu'
            '/perl/5.28.1 /usr/local/share/perl/5.28.1 /usr/lib/'
            'x86_64-linux-gnu/perl5/5.28 /usr/share/perl5 '
            '/usr/lib/x86_64-linux-gnu/perl/5.28 /usr/share/perl/5.28 '
            '/usr/local/lib/site_perl /usr/lib/x86_64-linux-gnu/perl-base) '
            'at Makefile.PL line 131.'], 1,
            MissingPerlFile('debian/perldl.conf', [
                '/<<PKGBUILDDIR>>/inc', '/etc/perl',
                '/usr/local/lib/x86_64-linux-gnu/perl/5.28.1',
                '/usr/local/share/perl/5.28.1',
                '/usr/lib/x86_64-linux-gnu/perl5/5.28',
                '/usr/share/perl5', '/usr/lib/x86_64-linux-gnu/perl/5.28',
                '/usr/share/perl/5.28', '/usr/local/lib/site_perl',
                '/usr/lib/x86_64-linux-gnu/perl-base']))
        self.run_test([
            'Can\'t open perl script "Makefile.PL": No such file or directory'
            ], 1, MissingPerlFile('Makefile.PL'))

    def test_missing_maven_artifacts(self):
        self.run_test([
            '[ERROR] Failed to execute goal on project byteman-bmunit5: Could '
            'not resolve dependencies for project '
            'org.jboss.byteman:byteman-bmunit5:jar:4.0.7: The following '
            'artifacts could not be resolved: '
            'org.junit.jupiter:junit-jupiter-api:jar:5.4.0, '
            'org.junit.jupiter:junit-jupiter-params:jar:5.4.0, '
            'org.junit.jupiter:junit-jupiter-engine:jar:5.4.0: '
            'Cannot access central (https://repo.maven.apache.org/maven2) '
            'in offline mode and the artifact '
            'org.junit.jupiter:junit-jupiter-api:jar:5.4.0 has not been '
            'downloaded from it before. -> [Help 1]'], 1,
            MissingMavenArtifacts([
                'org.junit.jupiter:junit-jupiter-api:jar:5.4.0',
                'org.junit.jupiter:junit-jupiter-params:jar:5.4.0',
                'org.junit.jupiter:junit-jupiter-engine:jar:5.4.0']))
        self.run_test([
            '[ERROR] Failed to execute goal on project opennlp-uima: Could '
            'not resolve dependencies for project '
            'org.apache.opennlp:opennlp-uima:jar:1.9.2-SNAPSHOT: Cannot '
            'access ApacheIncubatorRepository '
            '(http://people.apache.org/repo/m2-incubating-repository/) in '
            'offline mode and the artifact '
            'org.apache.opennlp:opennlp-tools:jar:debian has not been '
            'downloaded from it before. -> [Help 1]'], 1,
            MissingMavenArtifacts(
                ['org.apache.opennlp:opennlp-tools:jar:debian']))
        self.run_test([
            '[ERROR] Failed to execute goal on project bookkeeper-server: '
            'Could not resolve dependencies for project '
            'org.apache.bookkeeper:bookkeeper-server:jar:4.4.0: Cannot '
            'access central (https://repo.maven.apache.org/maven2) in '
            'offline mode and the artifact io.netty:netty:jar:debian '
            'has not been downloaded from it before. -> [Help 1]'], 1,
            MissingMavenArtifacts(['io.netty:netty:jar:debian']))
        self.run_test([
            '[ERROR] Unresolveable build extension: Plugin '
            'org.apache.felix:maven-bundle-plugin:2.3.7 or one of its '
            'dependencies could not be resolved: Cannot access central '
            '(https://repo.maven.apache.org/maven2) in offline mode and '
            'the artifact org.apache.felix:maven-bundle-plugin:jar:2.3.7 '
            'has not been downloaded from it before. @'], 1,
            MissingMavenArtifacts(
                ['org.apache.felix:maven-bundle-plugin:2.3.7']))
        self.run_test([
            '[ERROR] Plugin org.apache.maven.plugins:maven-jar-plugin:2.6 '
            'or one of its dependencies could not be resolved: Cannot access '
            'central (https://repo.maven.apache.org/maven2) in offline mode '
            'and the artifact '
            'org.apache.maven.plugins:maven-jar-plugin:jar:2.6 has not been '
            'downloaded from it before. -> [Help 1]'], 1,
            MissingMavenArtifacts(
                ['org.apache.maven.plugins:maven-jar-plugin:2.6']))

        self.run_test([
            '[FATAL] Non-resolvable parent POM for '
            'org.joda:joda-convert:2.2.1: Cannot access central '
            '(https://repo.maven.apache.org/maven2) in offline mode '
            'and the artifact org.joda:joda-parent:pom:1.4.0 has not '
            'been downloaded from it before. and \'parent.relativePath\' '
            'points at wrong local POM @ line 8, column 10'], 1,
            MissingMavenArtifacts(['org.joda:joda-parent:pom:1.4.0']))

        self.run_test([
            '[ivy:retrieve] \t\t:: '
            'com.carrotsearch.randomizedtesting#junit4-ant;'
            '${/com.carrotsearch.randomizedtesting/junit4-ant}: not found'], 1,
            MissingMavenArtifacts([
                'com.carrotsearch.randomizedtesting:junit4-ant:jar:debian']))

    def test_maven_errors(self):
        self.run_test([
            '[ERROR] Failed to execute goal '
            'org.apache.maven.plugins:maven-jar-plugin:3.1.2:jar '
            '(default-jar) on project xslthl: Execution default-jar of goal '
            'org.apache.maven.plugins:maven-jar-plugin:3.1.2:jar failed: '
            'An API incompatibility was encountered while executing '
            'org.apache.maven.plugins:maven-jar-plugin:3.1.2:jar: '
            'java.lang.NoSuchMethodError: '
            '\'void org.codehaus.plexus.util.DirectoryScanner.'
            'setFilenameComparator(java.util.Comparator)\''], 1, None)

    def test_dh_missing_uninstalled(self):
        self.run_test([
            'dh_missing --fail-missing',
            'dh_missing: usr/share/man/man1/florence_applet.1 exists in '
            'debian/tmp but is not installed to anywhere',
            'dh_missing: usr/lib/x86_64-linux-gnu/libflorence-1.0.la exists '
            'in debian/tmp but is not installed to anywhere',
            'dh_missing: missing files, aborting'], 3,
            DhMissingUninstalled(
                'usr/lib/x86_64-linux-gnu/libflorence-1.0.la'))

    def test_dh_until_unsupported(self):
        self.run_test([
            'dh: The --until option is not supported any longer (#932537). '
            'Use override targets instead.'], 1,
            DhUntilUnsupported())

    def test_missing_xml_entity(self):
        self.run_test([
            'I/O error : Attempt to load network entity '
            'http://www.oasis-open.org/docbook/xml/4.5/docbookx.dtd'],
            1, MissingXmlEntity(
                'http://www.oasis-open.org/docbook/xml/4.5/docbookx.dtd'))

    def test_ccache_error(self):
        self.run_test([
            'ccache: error: Failed to create directory '
            '/sbuild-nonexistent/.ccache/tmp: Permission denied'],
            1, CcacheError(
                'Failed to create directory '
                '/sbuild-nonexistent/.ccache/tmp: Permission denied'))

    def test_dh_addon_load_failure(self):
        self.run_test([
            'dh: unable to load addon nodejs: '
            'Debian/Debhelper/Sequence/nodejs.pm did not return a true '
            'value at (eval 11) line 1.'], 1,
            DhAddonLoadFailure(
                'nodejs', 'Debian/Debhelper/Sequence/nodejs.pm'))

    def test_missing_library(self):
        self.run_test([
            '/usr/bin/ld: cannot find -lpthreads'], 1,
            MissingLibrary('pthreads'))
        self.run_test([
            "./testFortranCompiler.f:4: undefined reference to `sgemm_'",
            ], 1)
        self.run_test([
            "writer.d:59: error: undefined reference to 'sam_hdr_parse_'",
            ], 1)

    def test_fpic(self):
        self.run_test([
            "/usr/bin/ld: pcap-linux.o: relocation R_X86_64_PC32 against "
            "symbol `stderr@@GLIBC_2.2.5' can not be used when making a "
            "shared object; recompile with -fPIC"], 1, None)

    def test_rspec(self):
        self.run_test([
            'rspec ./spec/acceptance/cookbook_resource_spec.rb:20 '
            '# Client API operations downloading a cookbook when the '
            'cookbook of the name/version is found downloads the '
            'cookbook to the destination'], 1, None)

    def test_multiple_definition(self):
        self.run_test([
            './dconf-paths.c:249: multiple definition of '
            '`dconf_is_rel_dir\'; client/libdconf-client.a(dconf-paths.c.o):'
            './obj-x86_64-linux-gnu/../common/dconf-paths.c:249: '
            'first defined here'], 1)

    def test_missing_ruby_gem(self):
        self.run_test([
            'Could not find gem \'childprocess (~> 0.5)\', which is '
            'required by gem \'selenium-webdriver\', in any of the sources.'],
            1, MissingRubyGem('childprocess', '0.5'))
        self.run_test([
            'Could not find gem \'rexml\', which is required by gem '
            '\'rubocop\', in any of the sources.'], 1, MissingRubyGem('rexml'))
        self.run_test([
            '/usr/lib/ruby/2.5.0/rubygems/dependency.rb:310:in `to_specs\': '
            'Could not find \'http-parser\' (~> 1.2.0) among 59 total gem(s) '
            '(Gem::MissingSpecError)'], 1,
            MissingRubyGem('http-parser', '1.2.0'))
        self.run_test([
            '/usr/lib/ruby/2.5.0/rubygems/dependency.rb:312:in `to_specs\': '
            'Could not find \'celluloid\' (~> 0.17.3) - did find: '
            '[celluloid-0.16.0] (Gem::MissingSpecVersionError)'], 1,
            MissingRubyGem('celluloid', '0.17.3'))
        self.run_test([
            '/usr/lib/ruby/2.5.0/rubygems/dependency.rb:312:in `to_specs\': '
            'Could not find \'i18n\' (~> 0.7) - did find: [i18n-1.5.3] '
            '(Gem::MissingSpecVersionError)'], 1,
            MissingRubyGem('i18n', '0.7'))
        self.run_test([
            '/usr/lib/ruby/2.5.0/rubygems/dependency.rb:310:in `to_specs\': '
            'Could not find \'sassc\' (>= 2.0.0) among 34 total gem(s) '
            '(Gem::MissingSpecError)'], 1,
            MissingRubyGem('sassc', '2.0.0'))
        self.run_test([
            '/usr/lib/ruby/2.7.0/bundler/resolver.rb:290:in '
            '`block in verify_gemfile_dependencies_are_found!\': '
            'Could not find gem \'rake-compiler\' in any of the gem sources '
            'listed in your Gemfile. (Bundler::GemNotFound)'], 1,
            MissingRubyGem('rake-compiler'))
        self.run_test([
            '/usr/lib/ruby/2.7.0/rubygems.rb:275:in `find_spec_for_exe\': '
            'can\'t find gem rdoc (>= 0.a) with executable rdoc '
            '(Gem::GemNotFoundException)'], 1,
            MissingRubyGem('rdoc', '0.a'))

    def test_missing_php_class(self):
        self.run_test([
            'PHP Fatal error:  Uncaught Error: Class '
            '\'PHPUnit_Framework_TestCase\' not found in '
            '/tmp/autopkgtest.gO7h1t/build.b1p/src/Horde_Text_Diff-'
            '2.2.0/test/Horde/Text/Diff/EngineTest.php:9'], 1,
            MissingPhpClass('PHPUnit_Framework_TestCase'))

    def test_missing_java_class(self):
        self.run_test("""\
Caused by: java.lang.ClassNotFoundException: org.codehaus.Xpp3r$Builder
\tat org.codehaus.strategy.SelfFirstStrategy.loadClass(lfFirstStrategy.java:50)
\tat org.codehaus.realm.ClassRealm.unsynchronizedLoadClass(ClassRealm.java:271)
\tat org.codehaus.realm.ClassRealm.loadClass(ClassRealm.java:247)
\tat org.codehaus.realm.ClassRealm.loadClass(ClassRealm.java:239)
\t... 46 more
""".splitlines(), 1, MissingJavaClass('org.codehaus.Xpp3r$Builder'))

    def test_install_docs_link(self):
        self.run_test("""\
dh_installdocs: --link-doc not allowed between sympow and sympow-data (one is \
arch:all and the other not)""".splitlines(), 1)

    def test_r_missing(self):
        self.run_test([
            "ERROR: dependencies ‘ellipsis’, ‘pkgload’ are not available "
            "for package ‘testthat’"], 1,
            MissingRPackage('ellipsis'))
        self.run_test([
            '  namespace ‘DBI’ 1.0.0 is being loaded, '
            'but >= 1.0.0.9003 is required'],
            1, MissingRPackage('DBI', '1.0.0.9003'))
        self.run_test([
            '  namespace ‘spatstat.utils’ 1.13-0 is already loaded, '
            'but >= 1.15.0 is required'], 1,
            MissingRPackage('spatstat.utils', '1.15.0'))
        self.run_test([
            'Error in library(zeligverse) : there is no package called '
            '\'zeligverse\''], 1, MissingRPackage('zeligverse'))
        self.run_test(
            ['there is no package called \'mockr\''], 1,
            MissingRPackage('mockr'))

    def test_mv_stat(self):
        self.run_test(
            ["mv: cannot stat '/usr/res/boss.png': No such file or directory"],
            1, MissingFile('/usr/res/boss.png'))
        self.run_test(
            ["mv: cannot stat 'res/boss.png': No such file or directory"],
            1)

    def test_dh_link_error(self):
        self.run_test(
            ['dh_link: link destination debian/r-cran-crosstalk/usr/lib/R/'
             'site-library/crosstalk/lib/ionrangeslider is a directory'], 1,
            DhLinkDestinationIsDirectory(
                'debian/r-cran-crosstalk/usr/lib/R/site-library/crosstalk/'
                'lib/ionrangeslider'))

    def test_go_test(self):
        self.run_test(
            ['FAIL\tgithub.com/edsrzf/mmap-go\t0.083s'], 1,
            FailedGoTest('github.com/edsrzf/mmap-go'))

    def test_debhelper_pattern(self):
        self.run_test(
            ['dh_install: Cannot find (any matches for) '
             '"server/etc/gnumed/gnumed-restore.conf" '
             '(tried in ., debian/tmp)'], 1,
            DebhelperPatternNotFound(
                'server/etc/gnumed/gnumed-restore.conf', 'install',
                ['.', 'debian/tmp']))

    def test_symbols(self):
        self.run_test([
            'dpkg-gensymbols: error: some symbols or patterns disappeared in '
            'the symbols file: see diff output below'], 1, None)

    def test_autoconf_macro(self):
        self.run_test([
            'configure.in:1802: error: possibly undefined macro: '
            'AC_CHECK_CCA'],
            1, MissingAutoconfMacro('AC_CHECK_CCA'))

    def test_config_status_input(self):
        self.run_test([
            'config.status: error: cannot find input file: '
            '`po/Makefile.in.in\''], 1,
            MissingConfigStatusInput('po/Makefile.in.in'))

    def test_jvm(self):
        self.run_test([
            'ERROR: JAVA_HOME is set to an invalid '
            'directory: /usr/lib/jvm/default-java/'], 1,
            MissingJVM())

    def test_cp(self):
        self.run_test([
            'cp: cannot stat '
            '\'/<<PKGBUILDDIR>>/debian/patches/lshw-gtk.desktop\': '
            'No such file or directory'], 1, None)

    def test_automake_input(self):
        self.run_test([
            'automake: error: cannot open < gtk-doc.make: '
            'No such file or directory'], 1,
            MissingAutomakeInput('gtk-doc.make'))

    def test_gettext_infrastructure(self):
        self.run_test([
            '*** error: gettext infrastructure mismatch: '
            'using a Makefile.in.in from gettext version '
            '0.19 but the autoconf macros are from gettext version 0.20'], 1,
            None)

    def test_shellcheck(self):
        self.run_test([
            ' ' * 40 +
            '^----^ SC2086: '
            'Double quote to prevent globbing and word splitting.'], 1, None)


class FindAptGetFailureDescriptionTests(unittest.TestCase):

    def run_test(self, lines, lineno, err=None):
        (offset, actual_line, actual_err) = find_apt_get_failure(
            lines)
        if lineno is not None:
            self.assertEqual(actual_line, lines[lineno-1])
            self.assertEqual(lineno, offset)
        else:
            self.assertIs(actual_line, None)
            self.assertIs(offset, None)
        if err:
            self.assertEqual(actual_err, err)
        else:
            self.assertIs(None, actual_err)

    def test_make_missing_rule(self):
        self.run_test(["""\
E: Failed to fetch http://janitor.debian.net/blah/Packages.xz  \
File has unexpected size (3385796 != 3385720). Mirror sync in progress? [IP]\
"""], 1, AptFetchFailure(
            'http://janitor.debian.net/blah/Packages.xz',
            'File has unexpected size (3385796 != 3385720). '
            'Mirror sync in progress? [IP]'))

    def test_missing_release_file(self):
        self.run_test(["""\
E: The repository 'https://janitor.debian.net blah/ Release' \
does not have a Release file.\
"""], 1, AptMissingReleaseFile(
            'http://janitor.debian.net/ blah/ Release'))

    def test_vague(self):
        self.run_test(["E: Stuff is broken"], 1, None)


class FindAutopkgtestFailureDescriptionTests(unittest.TestCase):

    def test_empty(self):
        self.assertEqual(
            (None, None, None, None),
            find_autopkgtest_failure_description([]))

    def test_no_match(self):
        self.assertEqual(
            (1, 'blalblala\n', None, None),
            find_autopkgtest_failure_description(['blalblala\n']))

    def test_unknown_error(self):
        self.assertEqual(
            (2, 'python-bcolz', None, 'Test python-bcolz failed: some error'),
            find_autopkgtest_failure_description(
                ['autopkgtest [07:58:03]: @@@@@@@@@@@@@@@@@@@@ summary\n',
                 'python-bcolz         FAIL some error\n']))

    def test_timed_out(self):
        error = AutopkgtestTimedOut()
        self.assertEqual(
            (2, 'unit-tests', error, 'timed out'),
            find_autopkgtest_failure_description(
                ['autopkgtest [07:58:03]: @@@@@@@@@@@@@@@@@@@@ summary\n',
                 'unit-tests           FAIL timed out']))

    def test_deps(self):
        error = AutopkgtestDepsUnsatisfiable(
            [('arg', '/home/janitor/tmp/tmppvupofwl/build-area/'
              'bcolz-doc_1.2.1+ds2-4~jan+lint1_all.deb'),
             ('deb', 'bcolz-doc'),
             ('arg', '/home/janitor/tmp/tmppvupofwl/build-area/python-'
              'bcolz-dbgsym_1.2.1+ds2-4~jan+lint1_amd64.deb'),
             ('deb', 'python-bcolz-dbgsym'),
             ('arg', '/home/janitor/tmp/'
              'tmppvupofwl/build-area/python-bcolz_1.2.1+ds2-4~jan'
              '+lint1_amd64.deb'),
             ('deb', 'python-bcolz'),
             ('arg', '/home/janitor/tmp/tmppvupofwl/build-area/'
              'python3-bcolz-dbgsym_1.2.1+ds2-4~jan+lint1_amd64.deb'),
             ('deb', 'python3-bcolz-dbgsym'),
             ('arg', '/home/janitor/tmp/tmppvupofwl/build-area/python3-'
              'bcolz_1.2.1+ds2-4~jan+lint1_amd64.deb'),
             ('deb', 'python3-bcolz'),
             (None, '/home/janitor/tmp/tmppvupofwl/build-area/'
              'bcolz_1.2.1+ds2-4~jan+lint1.dsc')])

        self.assertEqual(
            (2, 'python-bcolz', error,
             'Test python-bcolz failed: Test dependencies are unsatisfiable. '
             'A common reason is that your testbed is out of date '
             'with respect to the archive, and you need to use a '
             'current testbed or run apt-get update or use -U.'),
            find_autopkgtest_failure_description(
                ['autopkgtest [07:58:03]: @@@@@@@@@@@@@@@@@@@@ summary\n',
                 'python-bcolz         FAIL badpkg\n',
                 'blame: arg:/home/janitor/tmp/tmppvupofwl/build-area/'
                 'bcolz-doc_1.2.1+ds2-4~jan+lint1_all.deb deb:bcolz-doc '
                 'arg:/home/janitor/tmp/tmppvupofwl/build-area/python-'
                 'bcolz-dbgsym_1.2.1+ds2-4~jan+lint1_amd64.deb '
                 'deb:python-bcolz-dbgsym arg:/home/janitor/tmp/'
                 'tmppvupofwl/build-area/python-bcolz_1.2.1+ds2-4~jan'
                 '+lint1_amd64.deb deb:python-bcolz arg:/home/janitor/'
                 'tmp/tmppvupofwl/build-area/python3-bcolz-dbgsym_1.2.1'
                 '+ds2-4~jan+lint1_amd64.deb deb:python3-bcolz-dbgsym '
                 'arg:/home/janitor/tmp/tmppvupofwl/build-area/python3-'
                 'bcolz_1.2.1+ds2-4~jan+lint1_amd64.deb deb:python3-'
                 'bcolz /home/janitor/tmp/tmppvupofwl/build-area/'
                 'bcolz_1.2.1+ds2-4~jan+lint1.dsc\n',
                 'badpkg: Test dependencies are unsatisfiable. '
                 'A common reason is that your testbed is out of date '
                 'with respect to the archive, and you need to use a '
                 'current testbed or run apt-get update or use -U.\n']))
        error = AutopkgtestDepsUnsatisfiable(
            [('arg', '/home/janitor/tmp/tmpgbn5jhou/build-area/cmake'
              '-extras_1.3+17.04.20170310-6~jan+unchanged1_all.deb'),
             ('deb', 'cmake-extras'),
             (None, '/home/janitor/tmp/tmpgbn5jhou/'
              'build-area/cmake-extras_1.3+17.04.20170310-6~jan.dsc')])
        self.assertEqual(
            (2, 'intltool', error,
             'Test intltool failed: Test dependencies are unsatisfiable. '
             'A common reason is that your testbed is out of date with '
             'respect to the archive, and you need to use a current testbed '
             'or run apt-get update or use -U.'),
            find_autopkgtest_failure_description([
                'autopkgtest [07:58:03]: @@@@@@@@@@@@@@@@@@@@ summary\n',
                'intltool             FAIL badpkg',
                'blame: arg:/home/janitor/tmp/tmpgbn5jhou/build-area/cmake'
                '-extras_1.3+17.04.20170310-6~jan+unchanged1_all.deb '
                'deb:cmake-extras /home/janitor/tmp/tmpgbn5jhou/'
                'build-area/cmake-extras_1.3+17.04.20170310-6~jan.dsc',
                'badpkg: Test dependencies are unsatisfiable. A common '
                'reason is that your testbed is out of date with respect '
                'to the archive, and you need to use a current testbed or '
                'run apt-get update or use -U.']))

    def test_stderr(self):
        error = AutopkgtestStderrFailure('some output')
        self.assertEqual(
            (6, 'intltool', error,
             'Test intltool failed due to unauthorized stderr output: '
             'some output'),
            find_autopkgtest_failure_description([
                'intltool            FAIL stderr: some output',
                'autopkgtest [20:49:00]: test intltool:'
                '  - - - - - - - - - - stderr - - - - - - - - - -',
                'some output',
                'some more output',
                'autopkgtest [20:49:00]: @@@@@@@@@@@@@@@@@@@@ summary',
                'intltool            FAIL stderr: some output',
                ]))
        self.assertEqual(
            (2, 'intltool', MissingCommand('ss'),
             '/tmp/bla: 12: ss: not found'),
            find_autopkgtest_failure_description([
                'autopkgtest [20:49:00]: test intltool:'
                '  - - - - - - - - - - stderr - - - - - - - - - -',
                '/tmp/bla: 12: ss: not found',
                'some more output',
                'autopkgtest [20:49:00]: @@@@@@@@@@@@@@@@@@@@ summary',
                'intltool            FAIL stderr: /tmp/bla: 12: ss: not found',
                ]))
        self.assertEqual(
            (2, 'command10', MissingCommand('uptime'),
             'Can\'t exec "uptime": No such file or directory at '
             '/usr/lib/nagios/plugins/check_uptime line 529.'),
            find_autopkgtest_failure_description([
                'autopkgtest [07:58:03]: @@@@@@@@@@@@@@@@@@@@ summary\n',
                'command10            FAIL stderr: Can\'t exec "uptime": '
                'No such file or directory at '
                '/usr/lib/nagios/plugins/check_uptime line 529.']))

    def test_testbed_failure(self):
        error = AutopkgtestTestbedFailure(
            'sent `copyup /tmp/autopkgtest.9IStGJ/build.0Pm/src/ '
            '/tmp/autopkgtest.output.icg0g8e6/tests-tree/\', got '
            '`timeout\', expected `ok...\'')
        self.assertEqual(
            (1, None, error, None),
            find_autopkgtest_failure_description(
                ['autopkgtest [12:46:18]: ERROR: testbed failure: sent '
                 '`copyup /tmp/autopkgtest.9IStGJ/build.0Pm/src/ '
                 '/tmp/autopkgtest.output.icg0g8e6/tests-tree/\', got '
                 '`timeout\', expected `ok...\'\n']))

    def test_testbed_failure_with_test(self):
        error = AutopkgtestTestbedFailure(
            'testbed auxverb failed with exit code 255')
        self.assertEqual(
            (4, 'phpunit', error, None),
            find_autopkgtest_failure_description("""\
Removing autopkgtest-satdep (0) ...
autopkgtest [06:59:00]: test phpunit: [-----------------------
PHP Fatal error:  Declaration of Wicked_TestCase::setUp() must \
be compatible with PHPUnit\\Framework\\TestCase::setUp(): void in \
/tmp/autopkgtest.5ShOBp/build.ViG/src/wicked-2.0.8/test/Wicked/\
TestCase.php on line 31
autopkgtest [06:59:01]: ERROR: testbed failure: testbed auxverb \
failed with exit code 255
Exiting with 16
""".splitlines(True)))

    def test_test_command_failure(self):
        self.assertEqual(
            (7, 'command2',
             MissingFile('/usr/share/php/Pimple/autoload.php'),
             'Cannot open file "/usr/share/php/Pimple/autoload.php".'),
            find_autopkgtest_failure_description("""\
Removing autopkgtest-satdep (0) ...
autopkgtest [01:30:11]: test command2: phpunit --bootstrap /usr/autoload.php
autopkgtest [01:30:11]: test command2: [-----------------------
PHPUnit 8.5.2 by Sebastian Bergmann and contributors.

Cannot open file "/usr/share/php/Pimple/autoload.php".

autopkgtest [01:30:12]: test command2: -----------------------]
autopkgtest [01:30:12]: test command2:  \
- - - - - - - - - - results - - - - - - - - - -
command2             FAIL non-zero exit status 1
autopkgtest [01:30:12]: @@@@@@@@@@@@@@@@@@@@ summary
command1             PASS
command2             FAIL non-zero exit status 1
Exiting with 4
""".splitlines(True)))

    def test_dpkg_failure(self):
        self.assertEqual(
            (8, 'runtestsuite', AutopkgtestDepChrootDisappeared(),  """\
W: /var/lib/schroot/session/unstable-amd64-\
sbuild-7fb1b836-14f9-4709-8584-cbbae284db97: \
Failed to stat file: No such file or directory"""),
            find_autopkgtest_failure_description("""\
autopkgtest [19:19:19]: test require: [-----------------------
autopkgtest [19:19:20]: test require: -----------------------]
autopkgtest [19:19:20]: test require:  \
- - - - - - - - - - results - - - - - - - - - -
require              PASS
autopkgtest [19:19:20]: test runtestsuite: preparing testbed
Get:1 file:/tmp/autopkgtest.hdIETy/binaries  InRelease
Ign:1 file:/tmp/autopkgtest.hdIETy/binaries  InRelease
autopkgtest [19:19:23]: ERROR: "dpkg --unpack \
/tmp/autopkgtest.hdIETy/4-autopkgtest-satdep.deb" failed with \
stderr "W: /var/lib/schroot/session/unstable-amd64-sbuild-\
7fb1b836-14f9-4709-8584-cbbae284db97: Failed to stat file: \
No such file or directory
""".splitlines(True)))

    def test_last_stderr_line(self):
        self.assertEqual(
            (11, 'unmunge', None,
             'Test unmunge failed: non-zero exit status 2'),
            find_autopkgtest_failure_description("""\
autopkgtest [17:38:49]: test unmunge: [-----------------------
munge: Error: Failed to access "/run/munge/munge.socket.2": \
No such file or directory
unmunge: Error: No credential specified
autopkgtest [17:38:50]: test unmunge: -----------------------]
autopkgtest [17:38:50]: test unmunge: \
 - - - - - - - - - - results - - - - - - - - - -
unmunge              FAIL non-zero exit status 2
autopkgtest [17:38:50]: test unmunge: \
 - - - - - - - - - - stderr - - - - - - - - - -
munge: Error: Failed to access "/run/munge/munge.socket.2": \
No such file or directory
unmunge: Error: No credential specified
autopkgtest [17:38:50]: @@@@@@@@@@@@@@@@@@@@ summary
unmunge              FAIL non-zero exit status 2
Exiting with 4
""".splitlines(True)))

    def test_python_error_in_output(self):
        self.assertEqual(
            (7, 'unit-tests-3', None,
             'builtins.OverflowError: mktime argument out of range'),
            find_autopkgtest_failure_description("""\
autopkgtest [14:55:35]: test unit-tests-3: [-----------------------
  File "twisted/test/test_log.py", line 511, in test_getTimezoneOffsetWithout
    self._getTimezoneOffsetTest("Africa/Johannesburg", -7200, -7200)
  File "twisted/test/test_log.py", line 460, in _getTimezoneOffsetTest
    daylight = time.mktime(localDaylightTuple)
builtins.OverflowError: mktime argument out of range
-------------------------------------------------------------------------------
Ran 12377 tests in 143.490s

143.4904797077179 12377 12377 1 0 2352
autopkgtest [14:58:01]: test unit-tests-3: -----------------------]
autopkgtest [14:58:01]: test unit-tests-3: \
 - - - - - - - - - - results - - - - - - - - - -
unit-tests-3         FAIL non-zero exit status 1
autopkgtest [14:58:01]: @@@@@@@@@@@@@@@@@@@@ summary
unit-tests-3         FAIL non-zero exit status 1
Exiting with 4
""".splitlines(True)))


class ParseBrzErrorTests(unittest.TestCase):

    def test_inconsistent_source_format(self):
        self.assertEqual(
            (InconsistentSourceFormat(),
                'Inconsistent source format between version and source '
                'format'),
            parse_brz_error(
                'Inconsistency between source format and version: version '
                'is not native, format is native.'))
