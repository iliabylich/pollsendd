use crate::Service;
use rustix::io::Errno;

pub enum Error<S>
where
    S: Service,
{
    AlreadyRunning,
    FailedToGetSystemdProvidedSocket,
    EmptySdListenFds,
    Errno(Errno),
    CustomError(S::Error),
    FailedToPollServer,
}

impl<S> core::fmt::Display for Error<S>
where
    S: Service,
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

impl<S> core::fmt::Debug for Error<S>
where
    S: Service,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl<S> core::error::Error for Error<S> where S: Service {}

impl<S> From<Errno> for Error<S>
where
    S: Service,
{
    fn from(err: Errno) -> Self {
        Self::Errno(err)
    }
}
