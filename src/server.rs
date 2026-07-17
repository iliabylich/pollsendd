use crate::{AsPollFd, Error, Service, client::Client};
use rustix::{
    event::{PollFd, PollFlags},
    fs::Mode,
    net::{AddressFamily, SocketAddrUnix, SocketType},
};
use std::{
    collections::HashMap,
    io::ErrorKind,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
};

pub struct Server<const REQUEST_SIZE: usize> {
    fd: OwnedFd,
    fd_to_client: HashMap<i32, Client<REQUEST_SIZE>>,
}

impl<const REQUEST_SIZE: usize> Server<REQUEST_SIZE> {
    fn new(fd: OwnedFd) -> Self {
        log::trace!("serverfd = {fd:?}");
        Self {
            fd,
            fd_to_client: HashMap::new(),
        }
    }

    pub(crate) fn new_with_systemd_socket<S>() -> Result<Self, Error<REQUEST_SIZE, S>>
    where
        S: Service<REQUEST_SIZE>,
    {
        Ok(Self::new(systemd_socket()?))
    }

    pub(crate) fn new_with_socket_path<S>(path: &str) -> Result<Self, Error<REQUEST_SIZE, S>>
    where
        S: Service<REQUEST_SIZE>,
    {
        Ok(Self::new(socket_at(path)?))
    }

    pub(crate) fn pollfds(&self) -> impl Iterator<Item = PollFd<'_>> {
        self.fd_to_client
            .values()
            .map(|client| client.as_poll_fd())
            .chain([self.as_poll_fd()])
    }

    pub(crate) fn accept<S>(&mut self) -> Result<i32, Error<REQUEST_SIZE, S>>
    where
        S: Service<REQUEST_SIZE>,
    {
        log::trace!("Accepting a new client");
        let fd = rustix::net::accept(&self.fd)?;
        let rawfd = fd.as_raw_fd();
        self.fd_to_client.insert(rawfd, Client::new(fd));
        Ok(rawfd)
    }

    pub(crate) fn broadcast(&mut self, buf: &[u8]) {
        log::trace!("Broadcasting to {} clients..", self.fd_to_client.len());
        for client in self.fd_to_client.values_mut() {
            client.push(buf.to_vec());
        }
    }

    pub(crate) fn read_client_once(&mut self, fd: i32) -> Option<[u8; REQUEST_SIZE]> {
        let client = self.fd_to_client.get_mut(&fd)?;
        match client.read_once() {
            Ok(req) => req,
            Err(err) => {
                log::error!("{err:?}");
                self.fd_to_client.remove(&fd);
                None
            }
        }
    }

    pub(crate) fn push_to_client(&mut self, fd: i32, buf: Vec<u8>) {
        let Some(client) = self.fd_to_client.get_mut(&fd) else {
            return;
        };
        client.push(buf);
    }

    pub(crate) fn write_client(&mut self, fd: i32) {
        let Some(client) = self.fd_to_client.get_mut(&fd) else {
            return;
        };
        if let Err(err) = client.write() {
            log::error!("{err:?}");
            self.fd_to_client.remove(&fd);
        }
    }

    pub(crate) fn remove_client(&mut self, fd: i32) {
        self.fd_to_client.remove(&fd);
    }

    pub(crate) fn contains_client(&self, fd: i32) -> bool {
        self.fd_to_client.contains_key(&fd)
    }
}

impl<const REQUEST_SIZE: usize> AsFd for Server<REQUEST_SIZE> {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<const REQUEST_SIZE: usize> AsRawFd for Server<REQUEST_SIZE> {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl<const REQUEST_SIZE: usize> AsPollFd for Server<REQUEST_SIZE> {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::IN)
    }
}

pub fn systemd_socket<const REQUEST_SIZE: usize, S>() -> Result<OwnedFd, Error<REQUEST_SIZE, S>>
where
    S: Service<REQUEST_SIZE>,
{
    let fds = sd_listen_fds::get().map_err(|_| Error::FailedToGetSystemdProvidedSocket)?;
    let (_name, fd) = fds.into_iter().next().ok_or(Error::EmptySdListenFds)?;

    log::trace!("Listening on a systemd socket");
    Ok(fd.into_std())
}

pub fn socket_at<const REQUEST_SIZE: usize, S>(
    socket_path: &str,
) -> Result<OwnedFd, Error<REQUEST_SIZE, S>>
where
    S: Service<REQUEST_SIZE>,
{
    const SOMAXCONN: i32 = 4_096;

    let addr = SocketAddrUnix::new(socket_path)?;
    ensure_addr_is_free(&addr)?;

    let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)?;
    match rustix::fs::unlink(socket_path) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(Error::Errno(err)),
    }
    rustix::net::bind(&fd, &addr)?;
    rustix::fs::chmod(socket_path, Mode::from_raw_mode(0o666))?;
    rustix::net::listen(&fd, SOMAXCONN)?;

    log::trace!("Listening on {socket_path}");
    Ok(fd)
}

fn ensure_addr_is_free<const REQUEST_SIZE: usize, S>(
    addr: &SocketAddrUnix,
) -> Result<(), Error<REQUEST_SIZE, S>>
where
    S: Service<REQUEST_SIZE>,
{
    let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)?;
    if rustix::net::connect(&fd, addr).is_ok() {
        Err(Error::AlreadyRunning)
    } else {
        Ok(())
    }
}
