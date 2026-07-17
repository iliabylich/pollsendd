use crate::Service;
use rustix::io::Errno;

pub enum Error<const REQUEST_SIZE: usize, S>
where
    S: Service<REQUEST_SIZE>,
{
    AlreadyRunning,
    FailedToGetSystemdProvidedSocket,
    EmptySdListenFds,
    Errno(Errno),
    CustomError(S::Error),
    FailedToPollServer,
}

impl<const REQUEST_SIZE: usize, S> core::fmt::Display for Error<REQUEST_SIZE, S>
where
    S: Service<REQUEST_SIZE>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "AlreadyRunning"),
            Self::FailedToGetSystemdProvidedSocket => write!(f, "FailedToGetSystemdProvidedSocket"),
            Self::EmptySdListenFds => write!(f, "EmptySdListenFds"),
            Self::Errno(err) => write!(f, "Errno({err})"),
            Self::CustomError(err) => write!(f, "CustomError({err})"),
            Self::FailedToPollServer => write!(f, "FailedToPolLServer"),
        }
    }
}

impl<const REQUEST_SIZE: usize, S> core::fmt::Debug for Error<REQUEST_SIZE, S>
where
    S: Service<REQUEST_SIZE>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl<const REQUEST_SIZE: usize, S> core::error::Error for Error<REQUEST_SIZE, S> where
    S: Service<REQUEST_SIZE>
{
}

impl<const REQUEST_SIZE: usize, S> From<Errno> for Error<REQUEST_SIZE, S>
where
    S: Service<REQUEST_SIZE>,
{
    fn from(err: Errno) -> Self {
        Self::Errno(err)
    }
}
