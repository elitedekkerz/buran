use std::net::{TcpStream};
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::io::{
    BufRead,
    BufReader,
    Write,
    BufWriter,
};

use crossbeam;

use UiMessage;

pub enum ClientMessage {
    Command(String),
}

pub fn client(client_rx: mpsc::Receiver<ClientMessage>, ui_tx: mpsc::Sender<UiMessage>) {
    println!("connecting to server");
    let mut stream = TcpStream::connect("netl.fi:1961").unwrap();
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    writer.write(&['\n' as u8]);
    writer.flush();
    println!("starting listener thread");
    crossbeam::scope(|scope| {
        let ui_tx_ = ui_tx.clone();
        scope.spawn(move|| {
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                ui_tx_.send(UiMessage::Log(format!("< {}", line))).unwrap();
            }
        });
        let ui_tx_ = ui_tx.clone();
        scope.spawn(move|| {
            loop {
                if let Ok(message) = client_rx.recv() {
                    match message {
                        ClientMessage::Command(text) => {
                            writer.write(format!("{}\n", text).as_ref()).unwrap();
                            writer.flush();
                            ui_tx.send(UiMessage::ClearCommandLine).unwrap();
                        },
                        _ => ()
                    }
                }
            }
        });
    });
}
