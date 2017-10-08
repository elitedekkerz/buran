use std::net::{
    TcpStream,
    Shutdown,
};
use std::ops::Deref;
use std::net::SocketAddr;
use std::sync::{
    mpsc,
    Arc,
};
use std::thread;
use std::time::Duration;
use std::io::{
    BufRead,
    BufReader,
    Write,
    BufWriter,
};

use crossbeam;
use regex::{
    Regex,
    Captures,
};

use UiMessage;

pub enum ClientMessage {
    Command(String),
}

enum ClientCommand {
    Connect(String),
    Disconnect,
    RawCommand(String),
}

fn parse_command(command: String) -> Result<ClientCommand,()> {
    lazy_static!{
        static ref connect_regex: Regex =       Regex::new("^connect (.+)").unwrap();
        static ref disconnect_regex: Regex =    Regex::new("^disconnect").unwrap();
        static ref rawcmd_regex: Regex =        Regex::new("^r (.+)").unwrap();
    }

    if let Some(captures) = connect_regex.captures(&command) {
        return Ok(ClientCommand::Connect(captures.get(1).unwrap().as_str().to_owned()));
    };

    if disconnect_regex.is_match(&command) { return Ok(ClientCommand::Disconnect); };

    if let Some(captures) = rawcmd_regex.captures(&command) {
        return Ok(ClientCommand::RawCommand(captures.get(1).unwrap().as_str().to_owned()));
    };

    Err(())
}

pub fn client(client_rx: mpsc::Receiver<ClientMessage>, ui_tx: mpsc::Sender<UiMessage>) {
    crossbeam::scope(|scope| {
        let mut reader_thread: Option<crossbeam::ScopedJoinHandle<()>> = None;
        let mut stream: Option<TcpStream> = None;

        loop {
            if let Ok(message) = client_rx.recv() { match message {
                ClientMessage::Command(text) => {
                    if let Ok(cmd) = parse_command(text) { match cmd {
                        ClientCommand::Connect(addr) => {
                            stream = Some(TcpStream::connect(addr).unwrap());
                            if let Some(ref mut stream) = stream {
                                stream.write(&['\n' as u8]);
                                stream.flush();
                                let stream_ = stream.try_clone().unwrap();
                                let ui_tx_ = ui_tx.clone();
                                reader_thread = Some(scope.spawn(move|| {
                                    let mut reader = BufReader::new(&stream_);
                                    let mut line = String::new();
                                    loop {
                                        let result = reader.read_line(&mut line).unwrap();
                                        if result == 0 { break; }
                                        ui_tx_.send(UiMessage::Log(format!("recv< {}", line))).unwrap();
                                        line.clear();
                                    }
                                }));
                            }
                        },
                        ClientCommand::Disconnect => {
                            if let Some(ref stream) = stream {
                                if let Ok(_) = stream.shutdown(Shutdown::Both) {
                                    ui_tx.send(UiMessage::Log("Disconnected from server\n".to_owned())).unwrap();
                                    if let Some(thread) = reader_thread {
                                        thread.join();
                                        reader_thread = None;
                                    }
                                } else {
                                    ui_tx.send(UiMessage::Log("Could not disconnect from server!\n".to_owned())).unwrap();
                                }
                            } else {
                                ui_tx.send(UiMessage::Log("Not connected to server!\n".to_owned())).unwrap();
                            }
                            stream = None;
                        },
                        ClientCommand::RawCommand(text) => {
                            if let Some(ref mut stream) = stream {
                                stream.write(format!("{}\necho a\n",text).as_bytes()).unwrap();
                                stream.flush();
                                ui_tx.send(UiMessage::Log(format!("send> {}\n", text))).unwrap();
                            }
                        }
                    }} else {
                        ui_tx.send(UiMessage::Log("Unrecognized command!\n".to_owned())).unwrap();
                    }
                },
                _ => ()
            }}
        }
    });
}
