use rustix::event::{PollFd, PollFlags};

/// A trait that your service must implement.
///
/// Must be passed to on of `Runtime`'s constructors.
pub trait Service {
    /// A custom error type
    type Error: core::error::Error;

    /// Returns an iterator over all file descriptors that your service
    /// managed internally + their interested (`PollFlags::{IN,OUT}`)
    ///
    /// # Errors
    ///
    /// May return an error that will be returned from `Runtime::poll()` once called
    fn pollfds(&mut self) -> Result<impl Iterator<Item = PollFd<'_>>, Self::Error>;

    /// Returns `true` if given `fd` belongs to your service
    fn owns_fd(&self, fd: i32) -> bool;

    /// Called by `Runtime` if `fd` (that belongs to you) is readable
    ///
    /// Additionally takes a `broadcast` function that may be optionally called to send
    /// an arbitrary chunk of bytes to all connected clients
    ///
    /// # Errors
    ///
    /// May return an error that will be returned from `Runtime::poll()` once called
    fn on_readable(&mut self, fd: i32, broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error>;

    /// Called by `Runtime` if `fd` (that belongs to you) is writable
    ///
    /// Additionally takes a `broadcast` function that may be optionally called to send
    /// an arbitrary chunk of bytes to all connected clients
    ///
    /// # Errors
    ///
    /// May return an error that will be returned from `Runtime::poll()` once called
    fn on_writable(&mut self, fd: i32, broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error>;

    /// Called by `Runtime` if `fd` (that belongs to you) got an error during `poll()`
    ///
    /// # Errors
    ///
    /// May return an error that will be returned from `Runtime::poll()` once called
    fn on_poll_error(&mut self, fd: i32, revents: PollFlags) -> Result<(), Self::Error>;

    /// When a new client connects you may want to automatically send "current state" of the service.
    /// To do that implement this function and return all bytes you want to be sent on connecting.
    fn state_as_serialized_bufs(&self) -> impl Iterator<Item = Vec<u8>>;
}
