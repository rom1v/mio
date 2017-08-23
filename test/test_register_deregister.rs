use {localhost, TryWrite};
use mio::*;
use mio::net::{TcpListener, TcpStream};
use bytes::SliceBuf;
use std::time::Duration;

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

struct TestHandler {
    server: TcpListener,
    client: TcpStream,
    state: usize,
}

impl TestHandler {
    fn new(srv: TcpListener, cli: TcpStream) -> TestHandler {
        TestHandler {
            server: srv,
            client: cli,
            state: 0,
        }
    }

    fn handle_read(&mut self, poll: &mut Poll, token: Token) {
        match token {
            SERVER => {
                trace!("handle_read; token=SERVER");
                let mut sock = self.server.accept().unwrap().0;
                sock.try_write_buf(&mut SliceBuf::wrap("foobar".as_bytes())).unwrap();
            }
            CLIENT => {
                trace!("handle_read; token=CLIENT");
                assert!(self.state == 0, "unexpected state {}", self.state);
                self.state = 1;
                poll.reregister(&self.client, CLIENT, Ready::writable(), PollOpt::level()).unwrap();
            }
            _ => panic!("unexpected token"),
        }
    }

    fn handle_write(&mut self, poll: &mut Poll, token: Token) {
        debug!("handle_write; token={:?}; state={:?}", token, self.state);

        assert!(token == CLIENT, "unexpected token {:?}", token);
        assert!(self.state == 1, "unexpected state {}", self.state);

        self.state = 2;
        poll.deregister(&self.client).unwrap();
        poll.deregister(&self.server).unwrap();
    }
}

#[test]
pub fn test_register_deregister() {
    let _ = ::env_logger::init();

    debug!("Starting TEST_REGISTER_DEREGISTER");
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);

    let addr = localhost();

    let server = TcpListener::bind(&addr).unwrap();

    info!("register server socket");
    poll.register(&server, SERVER, Ready::readable(), PollOpt::edge()).unwrap();

    let client = TcpStream::connect(&addr).unwrap();

    // Register client socket only as writable
    poll.register(&client, CLIENT, Ready::readable(), PollOpt::level()).unwrap();

    let mut handler = TestHandler::new(server, client);

    loop {
        poll.poll(&mut events, None).unwrap();

        if let Some(event) = events.get(0) {
            if event.readiness().is_readable() {
                handler.handle_read(&mut poll, event.token());
            }

            if event.readiness().is_writable() {
                handler.handle_write(&mut poll, event.token());
                break;
            }
        }
    }

    poll.poll(&mut events, Some(Duration::from_millis(100))).unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
pub fn test_register_with_no_readable_writable_is_ok() {
    let poll = Poll::new().unwrap();
    let addr = localhost();

    let sock = TcpListener::bind(&addr).unwrap();

    poll.register(&sock, Token(0), Ready::empty(), PollOpt::edge()).unwrap();

    poll.reregister(&sock, Token(0), Ready::readable(), PollOpt::edge()).unwrap();

    poll.reregister(&sock, Token(0), Ready::empty(), PollOpt::edge()).unwrap();
}
