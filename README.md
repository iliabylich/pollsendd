### `pollsendd`

This library provides a set of traits and types to simplify building a daemon that listens on a UNIX socket and is managed by systemd.

First, you need to define you own `impl Service` and your own error type:

```rust,no_run
use pollsendd::Service;
use rustix::event::{PollFd, PollFlags};
use std::os::fd::{AsRawFd, OwnedFd};

struct MyService {
    fd1: OwnedFd,
    fd2: OwnedFd,
}
impl Service for MyService {
    type Error = TimerError;

    fn pollfds(&mut self) -> Result<impl Iterator<Item = PollFd<'_>>, Self::Error> {
        // return an iterator over ALL file descriptors that you daemon manages
        // and specify what you want to eventually happen to them (become readable or writable)
        Ok([
            PollFd::new(&self.fd1, PollFlags::IN),
            PollFd::new(&self.fd2, PollFlags::OUT),
        ].into_iter())
    }

    fn owns_fd(&self, fd: i32) -> bool {
        // return true if given `fd` belongs to you
        [self.fd1.as_raw_fd(), self.fd2.as_raw_fd()].contains(&fd)
    }

    fn on_readable(
        &mut self,
        fd: i32,
        mut broadcast: impl FnMut(&[u8]),
    ) -> Result<(), Self::Error> {
        // here given `fd` belongs to you and it's guaranteed to be readable, feel free to read from it
        if fd == self.fd1.as_raw_fd() {
            let mut buf = [0; 1_024];
            rustix::io::read(&self.fd1, &mut buf)?;
            // process it somehow and (if needed) broadcast it to ALL clients connected to exposed UNIX socket
            if let Some(data_to_broadcast) = self.what_i_want_to_broadcast() {
                (broadcast)(&data_to_broadcast);
            }
        }

        Ok(())
    }

    fn on_writable(&mut self, _fd: i32, _broadcast: impl FnMut(&[u8])) -> Result<(), Self::Error> {
        // same story here, `fd` is yours, it is writable, `broadcast` is available
        Ok(())
    }

    fn on_poll_error(&mut self, _fd: i32, revents: PollFlags) -> Result<(), Self::Error> {
        // called if `poll()` (that is used internally) returned an error for some of your FDs
        // you should probably de-activate it or reconnect
        Ok(())

        // returning an error will make it pop out of `Runtime::poll()` (see below)
    }

    fn state_as_serialized_bufs(&self) -> impl Iterator<Item = Vec<u8>> {
        // when a new client connects this methods is called
        // and every returned blob of bytes is sent to client
        std::iter::once(b"hello".to_vec())
    }
}

// and an error type
#[derive(Debug)]
struct MyError;
impl core::fmt::Display for MyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}
impl core::error::Error for ClientError {}
```

Then you can do the following:

```rust,no_run
let service = MyService::new();
let mut rt = Runtime::new_with_socket_path("/run/your-app-dev.sock", service).unwrap();
// or, if you want systemd to manage your socket (relies on `sd_listen_fds`)
let mut rt = Runtime::new_with_systemd_socket(service).unwrap();

// and then simply do
loop {
    rt.poll().unwrap();
}
```

If you specified a socket path manually, run the app and you'll be able to connect to this UNIX socket:

1. on connect you'll receive what is returned `state_as_serialized_bufs()`
2. on every `broadcast` you'll receive the data

If you construct a runtime with `Runtime::new_with_systemd_socket()` [`sd_listen_fds`](https://www.freedesktop.org/software/systemd/man/latest/sd_listen_fds.html) will be called and then you need two systemd files:

```
# your-service.socket
[Unit]
Description=<Your service description> socket

[Socket]
ListenStream=/run/your-service-systemd.sock

[Install]
WantedBy=sockets.target
```

and

```
[Unit]
Description=<Your service description> service
Requires=your-service.socket

[Service]
Type=simple
ExecStart=/path/to/binary
```

Make sure to activate only the `.socket` file and systemd will automatically listen on this UNIX socket for you, and once some client connects it will spawn the server, pass FD to the binary and then app will do the rest of the work.
