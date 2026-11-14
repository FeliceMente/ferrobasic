#!/usr/bin/env python3
"""Linux PTY integration tests, stdlib only. Pass the built ferro executable."""
import fcntl
import os
import re
import select
import signal
import struct
import subprocess
import sys
import termios
import tempfile
import time
import unittest

FERRO = os.path.abspath(sys.argv.pop(1))
# Ctrl+Alt+D as a terminal sends the chord: both bytes in one write. Esc and
# Ctrl+D typed separately arrive apart and do not signal EOF.
EXPLICIT_EOF = b"\x1b\x04"
CLEAR_BUFFER = b"\x1b[1;5H\x1b[1;6F\x1b[3~"  # Ctrl+Home, Ctrl+Shift+End, Delete


class Terminal:
    def __init__(self, *, stdin=True, stdout=False, stderr=True, term="xterm", respond=True, close_stdout=False):
        self.master, self.slave = os.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 100, 0, 0))
        self.respond = respond
        self.home = tempfile.TemporaryDirectory(prefix="ferro-repl-test-")
        self.before = termios.tcgetattr(self.slave)
        self.tty_input = stdin
        self.terminal = b""
        self.out = b""
        self.err = b""

        def setup():
            os.setsid()
            fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)
            if close_stdout:
                os.close(1)

        env = dict(os.environ, HOME=self.home.name)
        if term is None:
            env.pop("TERM", None)
        else:
            env["TERM"] = term
        # A private HOME also exposes any accidental history-file creation.
        self.process = subprocess.Popen(
            [FERRO, "repl"], stdin=self.slave if stdin else subprocess.PIPE,
            stdout=self.slave if stdout else subprocess.PIPE,
            stderr=self.slave if stderr else subprocess.PIPE,
            preexec_fn=setup, env=env,
        )

    def pump(self, duration=0.15):
        deadline = time.monotonic() + duration
        streams = {self.master: "terminal"}
        if self.process.stdout:
            streams[self.process.stdout.fileno()] = "out"
        if self.process.stderr:
            streams[self.process.stderr.fileno()] = "err"
        while time.monotonic() < deadline:
            ready, _, _ = select.select(list(streams), [], [], max(0, deadline - time.monotonic()))
            for fd in ready:
                data = os.read(fd, 65536)
                if not data:
                    streams.pop(fd)
                    continue
                name = streams[fd]
                setattr(self, name, getattr(self, name) + data)
                # Emulate a terminal's answer to cursor-position requests.
                if self.respond and name == "terminal" and b"\x1b[6n" in data:
                    os.write(self.master, b"\x1b[1;1R" * data.count(b"\x1b[6n"))

    def wait_for(self, predicate, timeout=5):
        deadline = time.monotonic() + timeout
        while not predicate():
            if time.monotonic() > deadline:
                raise AssertionError((self.terminal, self.out, self.err, self.process.poll()))
            self.pump(0.05)

    def send(self, data):
        if self.tty_input:
            os.write(self.master, data)
        else:
            self.process.stdin.write(data)
            self.process.stdin.flush()
        self.pump()

    def finish(self, code=0):
        self.wait_for(lambda: self.process.poll() is not None)
        self.pump()
        assert self.process.returncode == code, (self.process.returncode, self.terminal, self.err)
        assert termios.tcgetattr(self.slave) == self.before, "terminal attributes were not restored"
        assert not os.listdir(self.home.name), "session history was written to disk"

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        for stream in [self.process.stdin, self.process.stdout, self.process.stderr]:
            if stream and not stream.closed:
                stream.close()
        os.close(self.master)
        os.close(self.slave)
        self.home.cleanup()


class ReplTerminalTests(unittest.TestCase):
    def start(self, **kwargs):
        terminal = Terminal(**kwargs)
        self.addCleanup(terminal.close)
        terminal.pump()
        return terminal

    def edited(self):
        t = self.start()
        t.wait_for(lambda: b"> " in t.terminal)
        self.assertIn(b"\x1b[", t.terminal)
        self.assertEqual(t.out, b"")
        return t

    def test_recall_edits_and_streams(self):
        t = self.edited()
        t.send(b"10+2\r")
        t.wait_for(lambda: t.out == b"12\n")
        t.send(b"\x1b[A")
        # Home, Delete, insert, End, Backspace, insert: 10+2 -> 20+3.
        t.send(b"\x1b[H\x1b[3~2\x1b[F\x7f3\r")
        t.wait_for(lambda: t.out == b"12\n23\n")
        t.send(b'Log("sentinel")\r')
        t.wait_for(lambda: t.out.endswith(b"sentinel\n"))
        t.send(b":quit\r")
        t.finish()
        self.assertEqual(t.out, b"12\n23\nsentinel\n")
        self.assertNotIn(b"\x1b", t.out)

    def test_multiline_sub_recall_edit_and_blank_line(self):
        t = self.edited()
        t.send(b"Sub F As Int\r")
        t.wait_for(lambda: b"... " in t.terminal)
        t.send(b"\rReturn 4\rEnd Sub\r")
        t.send(b"\x1b[A")
        # Recalled multiline input starts on its first line. Move to Return.
        t.send(b"\x1b[B\x1b[B\x1b[F\x7f7\r")
        t.send(b"F()\r")
        t.wait_for(lambda: t.out == b"7\n")
        t.send(b'Log("sentinel")\r')
        t.wait_for(lambda: t.out.endswith(b"sentinel\n"))
        t.send(b":quit\r")
        t.finish()
        self.assertEqual(t.out, b"7\nsentinel\n")

    def test_multiline_block_recall(self):
        t = self.edited()
        t.send(b"For i = 1 To 2\rLog(i)\rNext\r")
        t.wait_for(lambda: t.out == b"1\n2\n")
        t.send(b"\x1b[A")
        # Replace recalled input with a sentinel, then wait positively for it.
        # Any accidental recall execution must precede the sentinel in stdout.
        t.send(CLEAR_BUFFER + b'Log("sentinel")\r')
        t.wait_for(lambda: t.out.endswith(b"sentinel\n"))
        self.assertEqual(t.out, b"1\n2\nsentinel\n")
        t.send(b"\x1b[A\x1b[A\r")
        t.wait_for(lambda: t.out == b"1\n2\nsentinel\n1\n2\n")
        t.send(b":quit\r")
        t.finish()

    def test_commands_reset_keeps_recall_and_cancel_excludes_input(self):
        t = self.edited()
        t.send(b"Log(41)\r:reset\rIf True Then\r:help\r:cancel\r")
        t.send(b"\x1b[A\r")
        t.wait_for(lambda: t.out.endswith(b"41\n") and t.out.count(b"41\n") == 2)
        self.assertIn(b":cancel", t.out)
        t.send(b":quit\r")
        t.finish()

    def test_eof_and_incomplete_eof(self):
        for text, code in [(b"", 0), (b":quit", 0), (b"If True Then\r\r", 3), (b"If True Then\r:cancel", 0), (b'"open\r', 3)]:
            with self.subTest(text=text):
                t = self.edited()
                t.send(text + EXPLICIT_EOF)
                t.finish(code)
                if code:
                    self.assertIn(b"<repl:1>", t.terminal)

    def test_ctrl_d_deletes_without_executing_or_exiting(self):
        t = self.edited()
        t.send(b"10+22\r")
        t.wait_for(lambda: t.out == b"32\n")
        # Recall, delete the final 2 with Ctrl+D, then explicitly submit 10+2.
        t.send(b"\x1b[A\x1b[F\x1b[D\x04\r")
        t.send(b'Log("sentinel")\r')
        t.wait_for(lambda: t.out.endswith(b"sentinel\n"))
        self.assertEqual(t.out, b"32\n12\nsentinel\n")
        t.send(b"\x04")  # Ctrl+D on the empty prompt still signals EOF.
        t.finish()

    def test_ctrl_d_empty_continuation_diagnoses_pending_input(self):
        t = self.edited()
        t.send(b"If True Then\r:help\r")
        t.wait_for(lambda: b"Show this help" in t.out)
        t.send(b"\x04")
        t.finish(3)
        self.assertIn(b"<repl:1>", t.terminal)

    def test_explicit_eof_finishes_below_multiline_buffer(self):
        t = self.edited()
        t.send(b"If True Then\rLog(1)\rLog(2)\r")
        t.send(b"\x1b[A\x1b[A" + EXPLICIT_EOF)
        t.finish(3)
        # Strip terminal controls, retaining CR/LF. The final painted input
        # must end before diagnostics, even when EOF was pressed on an earlier row.
        painted = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", t.terminal)
        self.assertIn(b"\r\n<repl:1>", painted)
        self.assertEqual(t.out, b"")

    def test_unix_ctrl_z_is_editor_undo(self):
        t = self.edited()
        t.send(b"10+2")
        t.send(b"\x7f")
        t.send(b"\x1a\r")  # Undo deletion, rather than suspending the process.
        t.send(b'Log("sentinel")\r')
        t.wait_for(lambda: t.out.endswith(b"sentinel\n"))
        self.assertEqual(t.out, b"12\nsentinel\n")
        t.send(b":quit\r")
        t.finish()

    def test_ctrl_c_at_prompt(self):
        t = self.edited()
        t.send(b"If True Then\r\x03")
        t.finish(130)

    def test_ctrl_c_during_execution(self):
        t = self.edited()
        t.send(b'Log("running")\rDo While True\rLoop\r')
        t.wait_for(lambda: t.out == b"running\n")
        # Raw mode must be off while BASIC runs; the terminal delivers SIGINT.
        self.assertEqual(termios.tcgetattr(t.slave), t.before)
        t.send(b"\x03")
        t.finish(-signal.SIGINT)

    def test_handled_compile_and_runtime_errors_restore_terminal(self):
        t = self.edited()
        t.send(b'missing\rDim n As Int = "bad"\rExitApplication2(17)\r')
        t.finish(17)
        self.assertIn(b"<repl:1>", t.terminal)
        self.assertIn(b"NumberFormatException", t.terminal)
        self.assertEqual(t.out, b"")

    def test_redirected_input_has_no_prompts_or_editing(self):
        for stdout in [False, True]:
            t = self.start(stdin=False, stdout=stdout)
            t.send(b'Log("ok")\nmissing\nLog("never")\n')
            t.process.stdin.close()
            t.finish(3)
            self.assertNotIn(b"\x1b", t.terminal + t.out + t.err)
            self.assertNotIn(b"> ", t.terminal + t.err)
            if not stdout:
                self.assertEqual(t.out, b"ok\n")
            self.assertNotIn(b"never", t.out + t.terminal)

    def test_redirected_stderr_uses_plain_interactive_reader(self):
        t = self.start(stderr=False)
        t.wait_for(lambda: t.err == b"> ")
        t.send(b"1+2\n:quit\n")
        t.finish()
        self.assertEqual(t.out, b"3\n")
        self.assertEqual(t.err, b"> > ")
        self.assertNotIn(b"\x1b", t.terminal + t.out + t.err)

    def test_unsupported_terminal_falls_back(self):
        for term in ["dumb", "unknown", "", None]:
            t = self.start(term=term)
            t.wait_for(lambda: b"> " in t.terminal)
            t.send(b"1+2\n:quit\n")
            t.finish()
            self.assertEqual(t.out, b"3\n")
            self.assertNotIn(b"\x1b", t.terminal)

    def test_cursor_query_failure_restores_terminal_and_stdout(self):
        t = self.start(respond=False)
        t.finish(2)
        self.assertIn(b"cannot read input", t.terminal)
        self.assertEqual(t.out, b"")

    def test_stdout_closed_before_exec(self):
        # Rust reopens closed stdio as /dev/null before main. Keep the
        # reviewer's retracted finding covered without an unnecessary workaround.
        t = self.start(close_stdout=True)
        t.wait_for(lambda: b"> " in t.terminal)
        t.send(b"Log(1)\r:quit\r")
        t.finish()
        self.assertNotIn(b"cannot read input", t.terminal)

    def test_other_redirected_combinations(self):
        for stdout in [False, True]:
            t = self.start(stdin=False, stdout=stdout, stderr=False)
            t.send(b'Log("ok")\n:quit\n')
            t.process.stdin.close()
            t.finish()
            self.assertEqual(t.err, b"")
            if not stdout:
                self.assertEqual(t.out, b"ok\n")
            self.assertNotIn(b"\x1b", t.terminal + t.out)
        t = self.start(stdout=True, stderr=False)
        t.wait_for(lambda: t.err == b"> ")
        t.send(b'Log("ok")\n:quit\n')
        t.finish()
        self.assertIn(b"ok\r\n", t.terminal)
        self.assertNotIn(b"\x1b", t.terminal + t.err)

    def test_all_terminal_streams(self):
        t = self.start(stdout=True)
        t.wait_for(lambda: b"> " in t.terminal)
        t.send(b'Log("basic-output")\r:quit\r')
        t.finish()
        self.assertIn(b"basic-output\r\n", t.terminal)


if __name__ == "__main__":
    unittest.main()
