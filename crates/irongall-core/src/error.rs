use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

/// Crate-wide result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Recoverable irongall failures. User-facing messages go through `Display`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    User(String),

    #[error("I/O error{context}: {source}", context = path_ctx(.path))]
    Io {
        #[source]
        source: io::Error,
        path: Option<PathBuf>,
    },

    #[error("failed to parse {what}: {source}")]
    Parse {
        what: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("command `{cmd}` failed{status}: {detail}", status = status_ctx(*.status))]
    Command {
        cmd: String,
        status: Option<ExitStatus>,
        detail: String,
    },

    #[error("apply finished with failures")]
    PartialApply,

    #[error("network error: {0}")]
    Network(String),
}

fn path_ctx(path: &Option<PathBuf>) -> String {
    match path {
        Some(p) => format!(" ({})", p.display()),
        None => String::new(),
    }
}

fn status_ctx(status: Option<ExitStatus>) -> String {
    match status {
        Some(s) => format!(" ({s})"),
        None => String::new(),
    }
}

impl Error {
    pub fn user(msg: impl Into<String>) -> Self {
        Self::User(msg.into())
    }

    pub fn io(source: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::Io {
            source,
            path: Some(path.into()),
        }
    }

    pub fn parse<E>(what: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Parse {
            what: what.into(),
            source: Box::new(source),
        }
    }

    /// Process exit code: 0 ok, 1 user/config, 2 partial apply.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::PartialApply => 2,
            _ => 1,
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::Io {
            source,
            path: None,
        }
    }
}
