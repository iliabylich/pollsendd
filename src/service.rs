use rustix::event::{PollFd, PollFlags};

/// A trait that your service must implement.
///
/// Must be passed to on of `Runtime`'s constructors.
pub trait Service<const REQUEST_SIZE: usize> {
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

    /// Called by `Runtime` when a new client connected. You may use this hook to "provision"
    /// a client with current state.
    ///
    /// # Errors
    ///
    /// May return an error that will be returned from `Runtime::poll()` once called
    fn on_client_connected(&mut self, write: impl FnMut(Vec<u8>)) -> Result<(), Self::Error>;

    /// Called by `Runtime` when a new client is readable and is ready to send a request to you.
    ///
    /// # Errors
    ///
    /// May return an error that will be returned from `Runtime::poll()` once called
    fn on_client_request(&mut self, request: [u8; REQUEST_SIZE]) -> Result<(), Self::Error>;
}
