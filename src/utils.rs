use std::fmt::{Display, Formatter, Result as FmtResult};
use std::result::Result as StdResult;
use std::error::Error as StdError;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::convert::From;

use futures::future::Future;

use native_tls::Error as TlsError;

use lastfm::error::Error as LastfmError;

// ----------------------------------------------------------------

pub type Result<T> = StdResult<T, Error>;
//pub type Response<T> = Box<Future<Item=T, Error=Error> + Send>;
pub type Data<'de, T> = Box<Future<Item = T, Error = Error> + Send + 'de>;

// ----------------------------------------------------------------

#[derive(Debug)]
pub enum Error {
    Build(IoError),
    Io(IoError),
    Tls(TlsError),
    Lastfm(LastfmError),
}

impl Error {
    pub fn build<E>(inner: E) -> Error
    where
        E: Into<Box<StdError + Send + Sync>>,
    {
        Error::Build(IoError::new(IoErrorKind::InvalidInput, inner))
    }

    pub fn io<E>(kind: IoErrorKind, inner: E) -> Error
    where
        E: Into<Box<StdError + Send + Sync>>,
    {
        Error::Io(IoError::new(kind, inner))
    }

    pub fn tls(inner: TlsError) -> Error {
        Error::Tls(inner)
    }

    pub fn lastfm(inner: LastfmError) -> Error {
        Error::Lastfm(inner)
    }
}

// ----------------------------------------------------------------

impl From<IoError> for Error {
    fn from(src: IoError) -> Self {
        Error::Io(src)
    }
}

impl From<TlsError> for Error {
    fn from(src: TlsError) -> Self {
        Error::tls(src)
    }
}

impl From<LastfmError> for Error {
    fn from(src: LastfmError) -> Self {
        Error::lastfm(src)
    }
}

// ----------------------------------------------------------------

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            Error::Build(ref inn) => write!(f, "Failed to build the client: {}", inn),
            Error::Io(ref inn) => write!(f, "I/O error: {}", inn),
            Error::Tls(ref inn) => write!(f, "HTTPS error: {}", inn),
            Error::Lastfm(ref inn) => write!(f, "Lastfm error: {}", inn),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Build(ref inn) => inn.description(),
            Error::Io(ref inn) => inn.description(),
            Error::Tls(ref inn) => inn.description(),
            Error::Lastfm(ref inn) => inn.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Build(ref inn) => Some(inn),
            Error::Io(ref inn) => Some(inn),
            Error::Tls(ref inn) => Some(inn),
            Error::Lastfm(ref inn) => Some(inn),
        }
    }
}
