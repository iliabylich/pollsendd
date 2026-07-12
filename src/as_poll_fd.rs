use rustix::event::PollFd;

pub trait AsPollFd {
    fn as_poll_fd(&self) -> PollFd<'_>;
}
