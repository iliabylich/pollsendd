use rustix::event::{PollFd, PollFlags};

pub trait Service {
    type Error: core::error::Error;

    fn pollfds(&mut self) -> Result<impl Iterator<Item = PollFd<'_>>, Self::Error>;

    fn owns_fd(&self, fd: i32) -> bool;

    fn on_readable(&mut self, fd: i32, broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error>;
    fn on_writable(&mut self, fd: i32, broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error>;
    fn on_poll_error(&mut self, fd: i32, revents: PollFlags) -> Result<(), Self::Error>;

    fn state_as_serialized_bufs(&self) -> impl Iterator<Item = Vec<u8>>;
}
