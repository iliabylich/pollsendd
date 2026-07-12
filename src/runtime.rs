use crate::{Error, Service, server::Server};
use rustix::event::PollFlags;
use std::os::fd::{AsFd, AsRawFd};

pub struct Runtime<S>
where
    S: Service,
{
    service: S,
    server: Server,
}

impl<S> Runtime<S>
where
    S: Service,
{
    pub fn new_with_systemd_socket(service: S) -> Result<Self, Error<S>> {
        Ok(Self {
            service,
            server: Server::new_with_systemd_socket()?,
        })
    }

    pub fn new_with_socket_path(path: &str, service: S) -> Result<Self, Error<S>> {
        Ok(Self {
            service,
            server: Server::new_with_socket_path(path)?,
        })
    }

    fn poll0(&mut self) -> Result<Vec<(i32, PollFlags)>, Error<S>> {
        let mut pollfds = self
            .server
            .pollfds()
            .chain(self.service.pollfds().map_err(Error::CustomError)?)
            .collect::<Vec<_>>();
        log::trace!("poll() {pollfds:?}...");
        rustix::event::poll(&mut pollfds, None)?;
        log::trace!("poll finished");
        Ok(pollfds
            .into_iter()
            .map(|pollfd| (pollfd.as_fd().as_raw_fd(), pollfd.revents()))
            .collect())
    }

    pub fn poll(&mut self) -> Result<(), Error<S>> {
        for (fd, revents) in self.poll0()? {
            let is_err = revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL);
            let is_readable = revents.contains(PollFlags::IN);
            let is_writable = revents.contains(PollFlags::OUT);

            if is_err {
                self.on_error(fd, revents)?;
            }

            if fd == self.server.as_raw_fd() && is_readable {
                self.server.accept()?;
            } else if self.server.contains_client(fd) && is_readable && self.server.read_client(fd)
            {
                for buf in self.service.state_as_serialized_bufs() {
                    self.server.write_client(fd, &buf);
                }
            } else if self.service.owns_fd(fd) {
                if is_readable {
                    self.service
                        .on_readable(fd, |buf| self.server.broadcast(buf))
                        .map_err(Error::CustomError)?;
                } else if is_writable {
                    self.service
                        .on_writable(fd, |buf| self.server.broadcast(buf))
                        .map_err(Error::CustomError)?;
                }
            }
        }
        Ok(())
    }

    fn on_error(&mut self, fd: i32, revents: PollFlags) -> Result<(), Error<S>> {
        if fd == self.server.as_raw_fd() {
            Err(Error::FailedToPollServer)
        } else if self.server.contains_client(fd) {
            log::error!("clientfd {fd} failed, removing it...");
            self.server.remove_client(fd);
            Ok(())
        } else if self.service.owns_fd(fd) {
            log::error!("custom slice {fd} failed");
            self.service
                .on_poll_error(fd, revents)
                .map_err(Error::CustomError)
        } else {
            Ok(())
        }
    }
}
