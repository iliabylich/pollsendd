use pollsendd::{Runtime, Service};
use rustix::{
    event::{PollFd, PollFlags},
    fs::Timespec,
    io::Errno,
    time::{
        Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, timerfd_create,
        timerfd_settime,
    },
};
use std::{
    fmt::Display,
    os::fd::{AsRawFd, OwnedFd},
};

struct Timer {
    fd: OwnedFd,
    tick: u64,
}

impl Timer {
    fn new() -> Self {
        let fd = timerfd_create(TimerfdClockId::Realtime, TimerfdFlags::empty()).unwrap();
        timerfd_settime(
            &fd,
            TimerfdTimerFlags::empty(),
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: 1,
                    tv_nsec: 0,
                },
            },
        )
        .unwrap();
        Self { fd, tick: 0 }
    }
}

impl Service<2> for Timer {
    type Error = TimerError;

    fn pollfds(&mut self) -> Result<impl Iterator<Item = PollFd<'_>>, Self::Error> {
        Ok(std::iter::once(PollFd::new(&self.fd, PollFlags::IN)))
    }

    fn owns_fd(&self, fd: i32) -> bool {
        self.fd.as_raw_fd() == fd
    }

    fn on_readable(
        &mut self,
        _fd: i32,
        mut broadcast: impl FnMut(&[u8]),
    ) -> Result<(), Self::Error> {
        let mut buf = [0; 8];
        let len = rustix::io::read(&self.fd, &mut buf)?;
        assert!(len == 8);

        self.tick += 1;
        let message = self.tick.to_be_bytes();
        (broadcast)(&message);

        Ok(())
    }

    fn on_writable(&mut self, _fd: i32, _broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error> {
        unreachable!()
    }

    fn on_poll_error(&mut self, _fd: i32, revents: PollFlags) -> Result<(), Self::Error> {
        Err(TimerError::FailedToPollTimer(revents))
    }

    fn on_client_connected(&mut self, mut write: impl FnMut(Vec<u8>)) -> Result<(), Self::Error> {
        (write)(b"hello".to_vec());
        Ok(())
    }

    fn on_client_request(&mut self, request: [u8; 2]) -> Result<(), Self::Error> {
        log::info!("client request: {request:?}");
        Ok(())
    }
}

#[derive(Debug)]
enum TimerError {
    Errno(Errno),
    FailedToPollTimer(PollFlags),
}
impl Display for TimerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Errno(err) => write!(f, "Errno({err})"),
            Self::FailedToPollTimer(revents) => write!(f, "FailedToPollTimer({revents:?})"),
        }
    }
}
impl core::error::Error for TimerError {}
impl From<Errno> for TimerError {
    fn from(err: Errno) -> Self {
        Self::Errno(err)
    }
}

fn main() {
    env_logger::init();

    let mut rt = Runtime::new_with_socket_path("/run/timer-demo.sock", Timer::new()).unwrap();

    loop {
        rt.poll().unwrap()
    }
}
