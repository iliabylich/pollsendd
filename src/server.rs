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

pub struct Server {
    fd: OwnedFd,
    fd_to_client: HashMap<i32, Client>,
}

impl Server {
    fn new(fd: OwnedFd) -> Self {
        log::trace!("serverfd = {fd:?}");
        Self {
            fd,
            fd_to_client: HashMap::new(),
        }
    }

    pub(crate) fn new_with_systemd_socket<S>() -> Result<Self, Error<S>>
    where
        S: Service,
    {
        Ok(Self::new(systemd_socket()?))
    }

    pub(crate) fn new_with_socket_path<S>(path: &str) -> Result<Self, Error<S>>
    where
        S: Service,
    {
        Ok(Self::new(socket_at(path)?))
    }

    pub(crate) fn pollfds(&self) -> impl Iterator<Item = PollFd<'_>> {
        self.fd_to_client
            .values()
            .map(|client| client.as_poll_fd())
            .chain([self.as_poll_fd()])
    }

    pub(crate) fn accept<S>(&mut self) -> Result<(), Error<S>>
    where
        S: Service,
    {
        log::trace!("Accepting a new client");
        let fd = rustix::net::accept(&self.fd)?;
        self.fd_to_client.insert(fd.as_raw_fd(), Client::new(fd));
        Ok(())
    }

    pub(crate) fn broadcast(&mut self, buf: &[u8]) {
        log::trace!("Broadcasting to {} clients..", self.fd_to_client.len());
        let mut fds_to_drop = vec![];
        for (fd, client) in &self.fd_to_client {
            if let Err(err) = client.write(buf) {
                log::error!("{err:?}");
                fds_to_drop.push(*fd);
            }
        }
        for fd in fds_to_drop {
            self.fd_to_client.remove(&fd);
        }
    }

    pub(crate) fn read_client(&mut self, fd: i32) -> bool {
        let Some(client) = self.fd_to_client.get(&fd) else {
            return false;
        };
        if let Err(err) = client.read() {
            log::error!("{err:?}");
            self.fd_to_client.remove(&fd);
            return false;
        }
        true
    }

    pub(crate) fn write_client(&mut self, fd: i32, buf: &[u8]) {
        let Some(client) = self.fd_to_client.get(&fd) else {
            return;
        };
        if let Err(err) = client.write(buf) {
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

impl AsFd for Server {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Server {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsPollFd for Server {
    fn as_poll_fd(&self) -> PollFd<'_> {
        PollFd::new(&self.fd, PollFlags::IN)
    }
}

pub fn systemd_socket<S>() -> Result<OwnedFd, Error<S>>
where
    S: Service,
{
    let fds = sd_listen_fds::get().map_err(|_| Error::FailedToGetSystemdProvidedSocket)?;
    let (_name, fd) = fds.into_iter().next().ok_or(Error::EmptySdListenFds)?;

    log::trace!("Listening on a systemd socket");
    Ok(fd.into_std())
}

pub fn socket_at<S>(socket_path: &str) -> Result<OwnedFd, Error<S>>
where
    S: Service,
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

fn ensure_addr_is_free<S>(addr: &SocketAddrUnix) -> Result<(), Error<S>>
where
    S: Service,
{
    let fd = rustix::net::socket(AddressFamily::UNIX, SocketType::STREAM, None)?;
    if rustix::net::connect(&fd, addr).is_ok() {
        Err(Error::AlreadyRunning)
    } else {
        Ok(())
    }
}
