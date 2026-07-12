use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};

use rustix::{
    event::{PollFd, PollFlags},
    io::Errno,
    net::{RecvFlags, SendFlags},
};

use crate::AsPollFd;

pub struct Client {
    fd: OwnedFd,
}

impl Client {
    pub(crate) const fn new(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub(crate) fn read(&self) -> Result<(), ClientError> {
        log::trace!("Reading 1 byte from client {}..", self.as_raw_fd());
        let (bytes_read, _) = rustix::net::recv(&self.fd, &mut [0; 1], RecvFlags::empty())
            .map_err(ClientError::Errno)?;
        if bytes_read == 1 {
            Ok(())
        } else {
            Err(ClientError::FailedToRead)
        }
    }

    pub(crate) fn write(&self, buf: &[u8]) -> Result<(), ClientError> {
        log::trace!("Writing to client {}..", self.as_raw_fd());
        let bytes_written =
            rustix::net::send(&self.fd, buf, SendFlags::NOSIGNAL).map_err(ClientError::Errno)?;
        if bytes_written == buf.len() {
            Ok(())
        } else {
            Err(ClientError::FailedToWrite)
        }
    }
}

impl AsFd for Client {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Client {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for Client {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(self, PollFlags::IN)
    }
}

#[derive(Debug)]
pub enum ClientError {
    Errno(Errno),
    FailedToRead,
    FailedToWrite,
}
impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Errno(errno) => write!(f, "Errno({errno})"),
            Self::FailedToRead => write!(f, "FailedToRead"),
            Self::FailedToWrite => write!(f, "FailedToWrite"),
        }
    }
}
impl core::error::Error for ClientError {}
