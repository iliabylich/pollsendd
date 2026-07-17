use crate::{Error, Service, server::Server};
use rustix::event::PollFlags;
use std::os::fd::{AsFd, AsRawFd};

pub struct Runtime<const REQUEST_SIZE: usize, S>
where
    S: Service<REQUEST_SIZE>,
{
    service: S,
    server: Server<REQUEST_SIZE>,
}

impl<const REQUEST_SIZE: usize, S> Runtime<REQUEST_SIZE, S>
where
    S: Service<REQUEST_SIZE>,
{
    /// Constructs a runtime with a server listening on a systemd-passed socket
    ///
    /// # Errors
    ///
    /// Returns an error if there no systemd socket is passed
    pub fn new_with_systemd_socket(service: S) -> Result<Self, Error<REQUEST_SIZE, S>> {
        Ok(Self {
            service,
            server: Server::new_with_systemd_socket()?,
        })
    }

    /// Constructs a runtime with a server listening on a given socket path
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to `socket()`->`listen()`
    pub fn new_with_socket_path(path: &str, service: S) -> Result<Self, Error<REQUEST_SIZE, S>> {
        Ok(Self {
            service,
            server: Server::new_with_socket_path(path)?,
        })
    }

    fn poll0(&mut self) -> Result<Vec<(i32, PollFlags)>, Error<REQUEST_SIZE, S>> {
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

    /// Polls the server, clients, and all FDs managed by your service
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// 1. `poll()` failed
    /// 2. `poll()` failed for serverfd
    /// 3. the server failed to `accept()` new client connection
    /// 4. your service returned an error from one of the dispatched methods
    pub fn poll(&mut self) -> Result<(), Error<REQUEST_SIZE, S>> {
        for (fd, revents) in self.poll0()? {
            let is_err = revents.intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL);
            let is_readable = revents.contains(PollFlags::IN);
            let is_writable = revents.contains(PollFlags::OUT);

            if is_err {
                self.on_error(fd, revents)?;
            }

            if fd == self.server.as_raw_fd() && is_readable {
                let clientfd = self.server.accept()?;
                self.service
                    .on_client_connected(|buf| self.server.push_to_client(clientfd, buf))
                    .map_err(Error::CustomError)?;
            } else if self.server.contains_client(fd) {
                if is_readable {
                    while let Some(request) = self.server.read_client_once(fd) {
                        self.service
                            .on_client_request(request)
                            .map_err(Error::CustomError)?;
                    }
                }
                if is_writable {
                    self.server.write_client(fd);
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

    fn on_error(&mut self, fd: i32, revents: PollFlags) -> Result<(), Error<REQUEST_SIZE, S>> {
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
