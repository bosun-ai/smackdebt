"""Black-box terminal checks using only Python's standard library."""
import os
import pty
import re
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time
import unittest
import fcntl
from pathlib import Path

BINARY = str(Path(sys.argv.pop(1)).resolve())


class Session:
    def __init__(self, home, args=(), options=None):
        options = options or {}
        width = options.get('width', 80)
        command = options.get('command', ('init',))
        self.master, self.slave = pty.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, width, 0, 0))
        self.before = termios.tcgetattr(self.slave)
        environment = dict(os.environ)
        for name in ('CI', 'NO_COLOR', 'CLICOLOR', 'CLICOLOR_FORCE', 'FORCE_COLOR'):
            environment.pop(name, None)
        environment.update(HOME=str(home), XDG_CONFIG_HOME=str(home / '.config'),
                           CODEX_HOME=str(home / '.codex'), CLAUDE_CONFIG_DIR=str(home / '.claude'),
                           TERM='xterm-256color', COLUMNS=str(width))
        environment.update(options.get('env', {}))
        self.process = subprocess.Popen([BINARY, *command, *args], cwd=home, env=environment,
                                        stdin=self.slave, stdout=self.slave, stderr=self.slave)
        self.output = b''

    def read(self):
        if select.select([self.master], [], [], .05)[0]:
            self.output += os.read(self.master, 65536)

    def wait_for(self, text):
        deadline = time.monotonic() + 5
        while text.encode() not in self.output and time.monotonic() < deadline:
            self.read()
            if self.process.poll() is not None:
                break
        assert text.encode() in self.output, self.output.decode(errors='replace')

    def send(self, keys):
        deadline = time.monotonic() + 2
        while termios.tcgetattr(self.slave)[3] & termios.ICANON and time.monotonic() < deadline:
            self.read()
        os.write(self.master, keys)
        self.read()

    def finish(self, code=0):
        deadline = time.monotonic() + 5
        while self.process.poll() is None and time.monotonic() < deadline:
            self.read()
        self.read()
        assert self.process.poll() == code, self.output.decode(errors='replace')
        after = termios.tcgetattr(self.slave)
        # macOS sets PENDIN after tcsetattr; it describes pending input, not raw mode.
        after[3] &= ~getattr(termios, 'PENDIN', 0)
        before = self.before.copy()
        before[3] &= ~getattr(termios, 'PENDIN', 0)
        assert after == before, f'terminal settings were not restored: {before!r} -> {after!r}'
        return self.output.decode(errors='replace')

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait()
        os.close(self.master)
        os.close(self.slave)


class SetupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)

    def session(self, *args, **kwargs):
        session = Session(self.home, args, kwargs)
        self.addCleanup(session.close)
        return session

    def skill(self, agent):
        return self.home / agent / 'skills/smackdebt/SKILL.md'

    def test_arrows_and_space_choose_agents_then_enter_installs(self):
        session = self.session()
        session.wait_for('Space')
        session.send(b' \x1b[B \r')
        session.finish()
        self.assertTrue(self.skill('.codex').exists())
        self.assertTrue(self.skill('.claude').exists())
        self.assertFalse(self.skill('.cursor').exists())

    def test_detected_agents_are_suggested_and_saved_choices_take_priority(self):
        (self.home / '.codex').mkdir()
        session = self.session()
        session.wait_for('Space')
        session.send(b'\r')
        session.finish()
        self.assertTrue(self.skill('.codex').exists())
        (self.home / '.cursor').mkdir()
        session = self.session()
        session.wait_for('Space')
        session.send(b'\r')
        self.assertIn('Already up to date', session.finish())
        self.assertFalse(self.skill('.cursor').exists())

    def test_escape_and_ctrl_c_cancel_without_writing(self):
        for key in (b'\x1b', b'\x03'):
            session = self.session()
            session.wait_for('Space')
            session.send(key)
            session.finish(130)
            self.assertEqual(list(self.home.iterdir()), [])
            self.assertIn(b'\x1b[?25h', session.output)

    def test_empty_selection_is_a_no_op(self):
        session = self.session()
        session.wait_for('Space')
        session.send(b'\r')
        self.assertIn('Nothing changed', session.finish())
        self.assertEqual(list(self.home.iterdir()), [])

    def test_uninstall_only_removes_checked_installations(self):
        self.session('--agent', 'codex', '--agent', 'cursor').finish()
        session = self.session('--uninstall')
        session.wait_for('Enter remove')
        session.send(b' \r')
        session.finish()
        self.assertTrue(self.skill('.codex').exists())
        self.assertFalse(self.skill('.cursor').exists())

    def test_ci_and_dumb_terminals_do_not_prompt(self):
        for environment in ({'CI': 'true'}, {'TERM': 'dumb'}):
            session = self.session(env=environment)
            text = session.finish(2)
            self.assertIn('--agent', text)
            self.assertNotIn('\x1b', text)

    def test_no_color_keeps_the_picker_without_styling(self):
        session = self.session(env={'NO_COLOR': '1'}, width=50)
        session.wait_for('Space')
        session.send(b' \r')
        text = session.finish()
        self.assertIsNone(re.search(r'\x1b\[[0-9;]*m', text))
        self.assertTrue(self.skill('.codex').exists())

    def test_saved_destination_survives_environment_changes_and_cancellation(self):
        destination = self.home / 'a deliberately long directory name' / 'another long directory' / 'skills'
        self.session('--agent', 'codex', '--dest', str(destination)).finish()
        saved = (self.home / '.config/smackdebt/agents.toml').read_bytes()
        session = self.session(width=50, env={'CODEX_HOME': str(self.home / 'elsewhere')})
        session.wait_for('Space')
        session.send(b'\x03')
        session.finish(130)
        self.assertEqual(saved, (self.home / '.config/smackdebt/agents.toml').read_bytes())
        self.assertTrue((destination / 'smackdebt/SKILL.md').exists())
        self.assertFalse((self.home / 'elsewhere').exists())

    def test_agent_detection_respects_configuration_overrides(self):
        root = self.home / 'custom-codex'
        root.mkdir()
        session = self.session(env={'CODEX_HOME': str(root)})
        session.wait_for('Space')
        session.send(b'\r')
        session.finish()
        self.assertTrue((root / 'skills/smackdebt/SKILL.md').exists())
        self.assertFalse(self.skill('.codex').exists())

    def test_help_respects_explicit_color_and_no_color(self):
        for environment in ({}, {'NO_COLOR': '1'}):
            session = self.session('--color', 'never', '--help', env=environment)
            text = session.finish()
            self.assertNotIn('\x1b', text)
            self.assertIn('Examples:', text)

    def test_analysis_progress_is_transient_and_json_stays_clean(self):
        import shutil
        import json
        real_git = shutil.which('git')
        (self.home / 'work.js').write_text('export const answer = () => 42;\n')
        for args in (['init', '-q'], ['add', 'work.js'],
                     ['-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'fixture']):
            subprocess.run([real_git, *args], cwd=self.home, check=True, capture_output=True)
        tools = self.home / '.bin'
        tools.mkdir()
        wrapper = tools / 'git'
        wrapper.write_text('#!' + sys.executable + '\nimport os,sys,time\n'
                           + 'if "log" in sys.argv: time.sleep(0.5)\n'
                           + 'os.execv(' + repr(real_git) + ', [' + repr(real_git) + '] + sys.argv[1:])\n')
        wrapper.chmod(0o755)
        env = {'PATH': str(tools) + os.pathsep + os.environ['PATH']}
        session = self.session(command=(), env=env)
        text = session.finish()
        self.assertIn('Analyzing code', text)
        self.assertIn('smackdebt ·', text)
        self.assertLess(text.rfind('Analyzing code'), text.find('smackdebt ·'))
        session = self.session('--json', command=(), env=env)
        text = session.finish()
        json.loads(text)
        self.assertNotIn('Analyzing code', text)


if __name__ == '__main__':
    unittest.main()
