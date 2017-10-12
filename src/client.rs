use std::net::{
    TcpStream,
    Shutdown,
};
use std::io::Write;
use std::ops::Deref;
use std::net::SocketAddr;
use std::sync::{
    mpsc,
    Arc,
};
use std::thread;
use std::time::Duration;

use regex::{
    Regex,
    Captures,
};

use UiMessage;
use servercomms::*;

pub enum GuiClientMessage {
    Command(String),
}

enum ClientCommsCommand {
    Connect(String),
    Disconnect,
    RawCommand(String),
}

fn handle_gui_message(message: &GuiClientMessage) -> Result<ClientCommsCommand,UiMessage> {
    match {
        match message {
            &GuiClientMessage::Command(ref text) => parse_cli_command(text)
        }
    } {
        Ok(r) => Ok(r),
        Err(()) => Err(UiMessage::Log("Invalid command!".to_owned())),
    }
}

fn parse_cli_command(command: &str) -> Result<ClientCommsCommand,()> {
    lazy_static!{
        static ref connect_regex: Regex =       Regex::new("^connect (.+)").unwrap();
        static ref disconnect_regex: Regex =    Regex::new("^disconnect").unwrap();
        static ref rawcmd_regex: Regex =        Regex::new("^r (.+)").unwrap();
    }

    if let Some(captures) = connect_regex.captures(&command) {
        return Ok(ClientCommsCommand::Connect(captures.get(1).unwrap().as_str().to_owned()));
    };

    if disconnect_regex.is_match(&command) { return Ok(ClientCommsCommand::Disconnect); };

    if let Some(captures) = rawcmd_regex.captures(&command) {
        return Ok(ClientCommsCommand::RawCommand(captures.get(1).unwrap().as_str().to_owned()));
    };

    Err(())
}

fn handle_server_response(response: &ServerResponse) -> UiMessage {
    use self::ServerResponse::*;
    use std::result::Result::Ok;

    match response {
        &Error(ref text) => UiMessage::Log(format!("Error: {}", text)),
        &ServerResponse::Ok => UiMessage::Log("ok".to_owned()),

        &GeneratorGet(factor,output) => UiMessage::Log(format!(
                        "Generator is set to {}% and power output is {} kW.",
                        factor*100.0,
                        output
                )),

        &RawResponse(ref text) => UiMessage::Log(format!("Raw command response:\n{}", text)),

        _ => UiMessage::Log(format!("Unimplemented server response {:?}!", response))
    }
}

pub fn client(client_rx: mpsc::Receiver<GuiClientMessage>, ui_tx: mpsc::Sender<UiMessage>) {
    let mut servercomm: Option<Box<ServerCommunicator>> = None; 
    let mut disconnect = false;

    loop {
        if let Ok(message) = client_rx.recv() {
            if let Some(ref mut servercomm) = servercomm { //If connected to server
                ui_tx.send(match handle_gui_message(&message) {
                    Err(e) => e,
                    Ok(c) => handle_server_response(&match c {
                        ClientCommsCommand::Connect(_) => {
                            ServerResponse::Error("Already connected to server!".to_owned())
                        },
                        ClientCommsCommand::Disconnect => {
                            let mut ret;
                            if let Ok(_) = servercomm.disconnect() {
                                ret = ServerResponse::Ok
                            } else {
                                ret = ServerResponse::Error("Could not disconnect from server!\nConnection closed.".to_owned())
                            }
                            disconnect = true;
                            ret
                        },
                        ClientCommsCommand::RawCommand(text) => {
                            servercomm.server_command(&ServerCommand::RawCommand(&text))
                        },
                        _ => ServerResponse::Error("Not implemented".to_owned())
                    })
                });
            } else { //If not connected to server
                ui_tx.send(match handle_gui_message(&message) {
                    Err(e) => e,
                    Ok(c) => handle_server_response(&match c {
                        ClientCommsCommand::Connect(addr) => {
                            let mut stream = TcpStream::connect(addr).unwrap();
                            stream.set_nonblocking(true).unwrap();
                            stream.write(&['\n' as u8]).unwrap();

                            servercomm = Some(Box::new(CmdlineCommunicator {
                                tcpstream: stream,
                            }));
                            ServerResponse::Ok
                        },
                        _ => ServerResponse::Error("Not connected to server!".to_owned()),
                    })
                });
            }
}
        if disconnect {
            servercomm = None;
        }
    }
}
