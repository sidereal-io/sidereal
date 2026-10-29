//! The real terminal: stdin for answers, stderr for prompts, and echo turned
//! off through termios for a password.
use super::Terminal;
use std::{
    io::{self, BufRead, Write},
    os::fd::RawFd,
};

pub struct Console<R, W> {
    fd: RawFd,
    input: R,
    prompts: W,
}

impl Console<io::StdinLock<'static>, io::Stderr> {
    /// Answers from stdin, prompts on stderr, so stdout holds only results.
    pub fn stdio() -> Self {
        Self::new(libc::STDIN_FILENO, io::stdin().lock(), io::stderr())
    }
}

impl<R: BufRead, W: Write> Console<R, W> {
    /// `fd` is the descriptor `input` reads from; its echo is what a hidden
    /// read turns off.
    pub fn new(fd: RawFd, input: R, prompts: W) -> Self {
        Self { fd, input, prompts }
    }
}

impl<R: BufRead, W: Write> Terminal for Console<R, W> {
    fn is_interactive(&self) -> bool {
        unsafe { libc::isatty(self.fd) == 1 }
    }

    fn prompt(&mut self, text: &str) -> io::Result<()> {
        self.prompts.write_all(text.as_bytes())?;
        self.prompts.flush()
    }

    fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        if self.input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
        Ok(Some(line))
    }

    fn read_hidden(&mut self) -> io::Result<Option<String>> {
        let Some(guard) = EchoOff::new(self.fd)? else {
            // Not a terminal, so there is no echo to turn off.
            return self.read_line();
        };
        let line = self.read_line();
        drop(guard);
        // The Enter key was not echoed either; end the prompt's line.
        self.prompt("\n")?;
        line
    }
}

/// Echo stays off while this lives. Dropping it restores the terminal as it
/// was, so an early return or a panic never leaves the terminal silent.
struct EchoOff {
    fd: RawFd,
    saved: libc::termios,
}

impl EchoOff {
    /// `None` when `fd` is not a terminal. Any other failure is an error: the
    /// command never reads a password while echo might be on.
    fn new(fd: RawFd) -> io::Result<Option<Self>> {
        let mut attributes = unsafe { std::mem::zeroed::<libc::termios>() };
        if unsafe { libc::tcgetattr(fd, &mut attributes) } != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ENOTTY) {
                return Ok(None);
            }
            return Err(hide_failed());
        }
        let saved = attributes;
        attributes.c_lflag &= !libc::ECHO;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &attributes) } != 0 {
            return Err(hide_failed());
        }
        Ok(Some(Self { fd, saved }))
    }
}

impl Drop for EchoOff {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.saved) };
    }
}

fn hide_failed() -> io::Error {
    io::Error::other("cannot turn off echo on the terminal")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::CStr,
        fs::File,
        io::{BufReader, Cursor},
        os::fd::{AsRawFd, FromRawFd},
    };

    /// A pseudo-terminal: the master end types, the slave end is stdin.
    fn pty() -> (File, File) {
        unsafe {
            let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            assert!(master >= 0);
            assert_eq!(libc::grantpt(master), 0);
            assert_eq!(libc::unlockpt(master), 0);
            let name = CStr::from_ptr(libc::ptsname(master)).to_owned();
            let slave = libc::open(name.as_ptr(), libc::O_RDWR | libc::O_NOCTTY);
            assert!(slave >= 0);
            (File::from_raw_fd(master), File::from_raw_fd(slave))
        }
    }

    fn pipe() -> (File, File) {
        let mut fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        unsafe { (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])) }
    }

    fn echoes(fd: RawFd) -> bool {
        let mut attributes = unsafe { std::mem::zeroed::<libc::termios>() };
        assert_eq!(unsafe { libc::tcgetattr(fd, &mut attributes) }, 0);
        attributes.c_lflag & libc::ECHO != 0
    }

    fn console(fd: RawFd, input: &str) -> Console<Cursor<Vec<u8>>, Vec<u8>> {
        Console::new(fd, Cursor::new(input.as_bytes().to_vec()), Vec::new())
    }

    fn file_console(file: File) -> Console<BufReader<File>, Vec<u8>> {
        Console::new(file.as_raw_fd(), BufReader::new(file), Vec::new())
    }

    #[test]
    fn the_echo_guard_turns_echo_off_until_it_drops() {
        let (_master, slave) = pty();
        let guard = EchoOff::new(slave.as_raw_fd()).unwrap().unwrap();
        assert!(!echoes(slave.as_raw_fd()));
        drop(guard);
        assert!(echoes(slave.as_raw_fd()));
    }

    #[test]
    fn a_hidden_read_on_a_terminal_answers_the_typed_line() {
        let (mut master, slave) = pty();
        master.write_all(b"secret-password\n").unwrap();
        let fd = slave.as_raw_fd();
        let mut console = file_console(slave);
        assert_eq!(
            console.read_hidden().unwrap().as_deref(),
            Some("secret-password")
        );
        assert!(echoes(fd));
    }

    #[test]
    fn a_hidden_read_on_a_terminal_ends_the_prompt_line() {
        let (mut master, slave) = pty();
        master.write_all(b"secret-password\n").unwrap();
        let mut console = file_console(slave);
        console.read_hidden().unwrap();
        assert_eq!(console.prompts, b"\n");
    }

    #[test]
    fn a_hidden_read_through_a_pipe_falls_back_to_a_plain_read() {
        let (read, mut write) = pipe();
        write.write_all(b"secret-password\n").unwrap();
        drop(write);
        let mut console = file_console(read);
        assert_eq!(
            console.read_hidden().unwrap().as_deref(),
            Some("secret-password")
        );
        assert!(console.prompts.is_empty());
    }

    #[test]
    fn a_hidden_read_is_refused_when_echo_cannot_be_checked() {
        let mut console = console(-1, "secret-password\n");
        assert!(console.read_hidden().is_err());
        assert_eq!(console.input.position(), 0);
    }

    #[test]
    fn a_read_strips_one_line_ending_and_nothing_else() {
        for (input, line) in [
            ("pass word\n", "pass word"),
            ("pass word\r\n", "pass word"),
            (" pass word \n", " pass word "),
            ("pass word\n\n", "pass word"),
            ("pass word", "pass word"),
        ] {
            assert_eq!(
                console(-1, input).read_line().unwrap().as_deref(),
                Some(line),
                "{input:?}"
            );
        }
    }

    #[test]
    fn a_read_tells_an_empty_line_from_a_closed_stdin() {
        assert_eq!(console(-1, "\n").read_line().unwrap().as_deref(), Some(""));
        assert_eq!(console(-1, "").read_line().unwrap(), None);
    }

    #[test]
    fn a_prompt_is_written_to_the_prompt_stream() {
        let mut console = console(-1, "");
        console.prompt("New password: ").unwrap();
        assert_eq!(console.prompts, b"New password: ");
    }

    #[test]
    fn only_a_terminal_is_interactive() {
        let (_master, slave) = pty();
        let (read, _write) = pipe();
        assert!(file_console(slave).is_interactive());
        assert!(!file_console(read).is_interactive());
    }
}
