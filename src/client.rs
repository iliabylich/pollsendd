use crate::AsPollFd;
use rustix::{
    event::{PollFd, PollFlags},
    io::Errno,
    net::{RecvFlags, SendFlags},
};
use std::{
    cmp::Ordering,
    collections::VecDeque,
    io::ErrorKind,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
};

pub struct Client<const REQUEST_SIZE: usize> {
    fd: OwnedFd,
    readbuf: [u8; REQUEST_SIZE],
    readpos: usize,
    writebuf: VecDeque<Vec<u8>>,
}

impl<const REQUEST_SIZE: usize> Client<REQUEST_SIZE> {
    pub(crate) const fn new(fd: OwnedFd) -> Self {
        Self {
            fd,
            readbuf: [0; _],
            readpos: 0,
            writebuf: VecDeque::new(),
        }
    }

    pub(crate) fn read_once(&mut self) -> Result<Option<[u8; REQUEST_SIZE]>, ClientError> {
        log::trace!("Reading from client {}..", self.as_raw_fd());

        let (bytes_read, _) = rustix::net::recv(
            &self.fd,
            &mut self.readbuf[self.readpos..],
            RecvFlags::empty(),
        )
        .map_err(ClientError::Errno)?;
        self.readpos = self
            .readpos
            .checked_add(bytes_read)
            .ok_or(ClientError::FailedToRead)?;

        match self.readpos.cmp(&REQUEST_SIZE) {
            Ordering::Less => Ok(None),
            Ordering::Equal => {
                let mut readbuf = [0; REQUEST_SIZE];
                std::mem::swap(&mut readbuf, &mut self.readbuf);
                Ok(Some(readbuf))
            }
            Ordering::Greater => Err(ClientError::FailedToRead),
        }
    }

    pub(crate) fn push(&mut self, buf: Vec<u8>) {
        self.writebuf.push_back(buf);
    }

    pub(crate) fn write(&mut self) -> Result<(), ClientError> {
        log::trace!("Writing to client {}..", self.as_raw_fd());

        loop {
            let Some(buf) = self.writebuf.front_mut() else {
                return Ok(());
            };

            let bytes_written = match rustix::net::send(&self.fd, buf, SendFlags::NOSIGNAL) {
                Ok(len) => len,
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(ClientError::Errno(err)),
            };
            *buf = buf
                .get(bytes_written..)
                .ok_or(ClientError::FailedToWrite)?
                .to_vec();

            if buf.is_empty() {
                self.writebuf.pop_front();
            }
        }

        Ok(())
    }
}

impl<const REQUEST_SIZE: usize> AsFd for Client<REQUEST_SIZE> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<const REQUEST_SIZE: usize> AsRawFd for Client<REQUEST_SIZE> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl<const REQUEST_SIZE: usize> AsPollFd for Client<REQUEST_SIZE> {
    fn as_poll_fd(&self) -> PollFd<'_> {
        let mut events = PollFlags::IN;
        if !self.writebuf.is_empty() {
            events |= PollFlags::OUT;
        }
        PollFd::new(self, events)
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
