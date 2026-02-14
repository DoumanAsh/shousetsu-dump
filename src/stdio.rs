use std::{io, fmt};

const NEWLINE: &[u8] = b"\n";

pub mod behavior {
    #[derive(Copy, Clone)]
    pub struct Result;
    #[derive(Copy, Clone)]
    pub struct Panic;
    #[derive(Copy, Clone)]
    pub struct Ignore;
}

pub struct Out<T, B> {
    inner: T,
    _behavior: B,
}

impl<T, B> Out<T, B> {
    pub fn new(inner: T, _behavior: B) -> Self {
        Self {
            inner,
            _behavior
        }
    }

    #[inline(always)]
    pub fn ignore_errors(self) -> Out<T, behavior::Ignore> {
        Out {
            inner: self.inner,
            _behavior: behavior::Ignore
        }
    }
}

impl<T: io::Write> Out<T, behavior::Result> {
    #[inline]
    pub fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> io::Result<()> {
        io::Write::write_fmt(&mut self.inner, args)?;
        io::Write::flush(&mut self.inner)
    }

    #[inline]
    pub fn write_fmtn(&mut self, args: fmt::Arguments<'_>) -> io::Result<()> {
        io::Write::write_fmt(&mut self.inner, args)?;
        io::Write::write(&mut self.inner, NEWLINE)?;
        io::Write::flush(&mut self.inner)
    }

    #[inline]
    pub fn write_newline(&mut self) -> io::Result<()> {
        io::Write::write(&mut self.inner, NEWLINE)?;
        io::Write::flush(&mut self.inner)
    }
}

impl<T: io::Write> Out<T, behavior::Ignore> {
    #[inline]
    pub fn write_newline(&mut self) {
        let _ = io::Write::write(&mut self.inner, NEWLINE);
        let _ = io::Write::flush(&mut self.inner);
    }

    #[inline]
    pub fn write_fmt(&mut self, args: fmt::Arguments<'_>) {
        let _ = io::Write::write_fmt(&mut self.inner, args);
        let _ = io::Write::flush(&mut self.inner);
    }

    #[inline]
    pub fn write_fmtn(&mut self, args: fmt::Arguments<'_>) {
        let _ = io::Write::write_fmt(&mut self.inner, args);
        let _ = io::Write::write(&mut self.inner, NEWLINE);
        let _ = io::Write::flush(&mut self.inner);
    }
}

impl<T: fmt::Debug, B> fmt::Debug for Out<T, B> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, fmt)
    }
}

pub struct In<T, B> {
    io: T,
    buffer: String,
    _behavior: B,
}

impl<T: io::BufRead, B> In<T, B> {
    #[inline]
    fn new(io: T, _behavior: B) -> Self {
        Self {
            io,
            buffer: String::new(),
            _behavior
        }
    }

    #[inline(always)]
    ///Current buffer content
    pub fn current_line(&self) -> &str {
        &self.buffer
    }

    #[inline(always)]
    pub fn panic_errors(self) -> In<T, behavior::Panic> {
        In {
            io: self.io,
            buffer: self.buffer,
            _behavior: behavior::Panic
        }
    }
}

impl<T: io::BufRead> In<T, behavior::Result> {
    ///Fills buffer, returning current content
    pub fn read_line(&mut self) -> io::Result<&str> {
        self.buffer.clear();
        self.io.read_line(&mut self.buffer)?;
        Ok(self.current_line())
    }
}

impl<T: io::BufRead> In<T, behavior::Panic> {
    ///Fills buffer, returning current content
    pub fn read_line(&mut self) -> &str {
        self.buffer.clear();
        if let Err(error) = self.io.read_line(&mut self.buffer) {
            panic!("read_line(): Unexpected I/O error: {}", error);
        }
        self.current_line()
    }
}

impl<T: fmt::Debug, B> fmt::Debug for In<T, B> {
    #[inline(always)]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.io, fmt)
    }
}

///Standard IO
pub struct Stdio {
    stdout: io::Stdout,
    stderr: io::Stderr,
    stdin: io::Stdin,
}

impl Stdio {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
            stdin: io::stdin(),
            stderr: io::stderr()
        }
    }

    #[inline]
    pub fn stdout(&self) -> Out<impl io::Write + fmt::Debug, behavior::Result> {
        Out::new(self.stdout.lock(), behavior::Result)
    }

    #[inline]
    pub fn stderr(&self) -> Out<impl io::Write + fmt::Debug, behavior::Result>{
        Out::new(self.stderr.lock(), behavior::Result)
    }

    #[inline]
    pub fn stdin(&self) -> In<impl io::BufRead + fmt::Debug, behavior::Result>{
        In::new(self.stdin.lock(), behavior::Result)
    }
}
