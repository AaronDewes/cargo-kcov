use std::io;
//use std::error;
use std::convert::From;
use std::fmt::Display;
use std::process::{exit, ExitStatus};
use std::str::Utf8Error;
use std::string::FromUtf8Error;

use serde_json;
use term::color::{GREEN, RED, WHITE, YELLOW};
use term::Attr;

use crate::stderr;

#[derive(Debug)]
pub enum Error {
    UnsupportedOS,
    KcovTooOld,
    KcovNotInstalled(io::Error),
    CannotRunCargo(io::Error),
    Utf8(Utf8Error),
    Json(Option<serde_json::Error>),
    CannotCreateCoverageDirectory(io::Error),
    Cargo {
        subcommand: &'static str,
        status: ExitStatus,
        stderr: Vec<u8>,
    },
    KcovFailed(io::Result<ExitStatus>),
    NoCoverallsId,
    CannotFindTestTargets(Option<io::Error>),
}

impl Error {
    fn description(&self) -> &str {
        match *self {
            Error::UnsupportedOS => "kcov cannot collect coverage on Windows.",
            Error::KcovTooOld => "kcov is too old. v30 or above is required.",
            Error::KcovNotInstalled(_) => "kcov not installed.",
            Error::CannotRunCargo(_) => "cannot run cargo",
            Error::Utf8(_) => "output is not UTF-8 encoded",
            Error::Json(_) => "cannot parse JSON",
            Error::Cargo { .. } => "cargo subcommand failure",
            Error::CannotCreateCoverageDirectory(_) => "cannot create coverage output directory",
            Error::KcovFailed(_) => "failed to get coverage",
            Error::NoCoverallsId => "missing environment variable TRAVIS_JOB_ID for coveralls",
            Error::CannotFindTestTargets(_) => "cannot find test targets",
        }
    }

    fn cause(&self) -> Option<&dyn Display> {
        match *self {
            Error::KcovNotInstalled(ref e)
            | Error::CannotRunCargo(ref e)
            | Error::CannotCreateCoverageDirectory(ref e)
            | Error::KcovFailed(Err(ref e)) => Some(e),
            Error::Utf8(ref e) => Some(e),
            Error::Json(ref e) => e.as_ref().map(|a| a as &dyn Display),
            Error::KcovFailed(Ok(ref e)) => Some(e),
            Error::CannotFindTestTargets(ref e) => e.as_ref().map(|a| a as &dyn Display),
            _ => None,
        }
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Error::Utf8(e.utf8_error())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(Some(e))
    }
}

impl Error {
    /// Prints the error message and quit.
    pub fn print_error_and_quit(&self) -> ! {
        let mut t = stderr::new();

        t.fg(RED).unwrap();
        t.attr(Attr::Bold).unwrap();
        t.write_all(b"error: ").unwrap();
        t.reset().unwrap();
        writeln!(t, "{}", self.description()).unwrap();

        if let Error::Cargo {
            subcommand,
            ref status,
            ref stderr,
        } = *self
        {
            t.fg(YELLOW).unwrap();
            t.attr(Attr::Bold).unwrap();
            t.write_all(b"note: ").unwrap();
            t.reset().unwrap();
            writeln!(t, "cargo {} exited with code {}", subcommand, status).unwrap();
            t.write_all(stderr).unwrap();
        }

        if let Some(cause) = self.cause() {
            t.fg(YELLOW).unwrap();
            t.attr(Attr::Bold).unwrap();
            t.write_all(b"caused by: ").unwrap();
            t.reset().unwrap();
            writeln!(t, "{}", cause).unwrap();
        }

        match *self {
            Error::KcovTooOld | Error::KcovNotInstalled(_) => {
                t.fg(GREEN).unwrap();
                t.attr(Attr::Bold).unwrap();
                t.write_all(b"note: ").unwrap();
                t.reset().unwrap();
                t.write_all(b"you may follow ").unwrap();
                t.attr(Attr::Underline(true)).unwrap();
                t.write_all(b"https://users.rust-lang.org/t/650").unwrap();
                t.reset().unwrap();
                t.write_all(b" to install kcov:\n\n").unwrap();

                #[cfg(target_os = "linux")]
                {
                    t.fg(WHITE).unwrap();
                    t.write_all(b"    $ ").unwrap();
                    t.reset().unwrap();
                    writeln!(t, "sudo apt-get install cmake g++ pkg-config jq\n").unwrap();

                    t.fg(WHITE).unwrap();
                    t.write_all(b"    $ ").unwrap();
                    t.reset().unwrap();
                    writeln!(t, "sudo apt-get install libcurl4-openssl-dev libelf-dev libdw-dev binutils-dev libiberty-dev\n").unwrap();
                }
                #[cfg(target_os = "macos")]
                {
                    t.fg(WHITE).unwrap();
                    t.write_all(b"    $ ").unwrap();
                    t.reset().unwrap();
                    writeln!(t, "brew install cmake jq\n").unwrap();
                }

                t.fg(WHITE).unwrap();
                t.write_all(b"    $ ").unwrap();
                t.reset().unwrap();
                writeln!(t, "cargo kcov --print-install-kcov-sh | sh").unwrap();
            }
            Error::CannotFindTestTargets(_) => {
                t.fg(GREEN).unwrap();
                t.attr(Attr::Bold).unwrap();
                t.write_all(b"note: ").unwrap();
                t.reset().unwrap();
                t.write_all(b"try a clean rebuild first:\n\n").unwrap();
                t.fg(WHITE).unwrap();
                t.write_all(b"    $ ").unwrap();
                t.reset().unwrap();
                writeln!(
                    t,
                    "cargo clean &&
        RUSTFLAGS=\"-C link-dead-code\" cargo test --no-run &&
        cargo kcov --no-clean-rebuild

                "
                )
                .unwrap();
            }
            _ => {}
        }

        exit(2);
    }
}
