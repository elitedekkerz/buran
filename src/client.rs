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
use regexes;

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

fn parse_cli_command(command: &str) -> Result<ClientCommsCommand,UiMessage> {
    if let Some(captures) = regexes::connect.captures(&command) {
        return Ok(ClientCommsCommand::Connect(captures.get(1).unwrap().as_str().to_owned()));
    };

    if regexes::disconnect.is_match(&command) { return Ok(ClientCommsCommand::Disconnect); };

    if let Some(captures) = regexes::rawcmd.captures(&command) {
        return Ok(ClientCommsCommand::RawCommand(captures.get(1).unwrap().as_str().to_owned()));
    };

    if let Some(captures) = regexes::radio.captures(&command) {
        let arg = captures.get(1).unwrap().as_str();
        if regexes::on.is_match(arg) {
            return Ok(ClientCommsCommand::RadioOn(true));
        } else if regexes::off.is_match(arg) {
            return Ok(ClientCommsCommand::RadioOn(false));
        } else if let Some(captures) = regexes::set.captures(arg) {
            let arg

    Err(UiMessage::LogLn("Unrecognized command!"))
}

fn handle_server_response(response: &ServerResponse) -> UiMessage {
    use self::ServerResponse::*;
    use std::result::Result::Ok;

    match response {
        &Error(ref text) => UiMessage::LogLn(format!("Error: {}", text)),
        &ServerResponse::Ok => UiMessage::LogLn("ok"),

        &GeneratorGet(factor,output) => UiMessage::LogLn(format!(
                        "Generator is set to {}% and power output is {} kW.",
                        factor*100.0,
                        output
                )),

        &RawResponse(ref text) => UiMessage::LogLn(format!("raw response: \n{}", text)),

        _ => UiMessage::LogLn(format!("Unimplemented server response {:?}!", response))
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
                            let mut stream = TcpStream::connect(addr);
                            if let Ok(mut stream) = stream {
                                stream.set_nonblocking(true).unwrap();
                                servercomm = Some(Box::new(CmdlineCommunicator {
                                    tcpstream: stream,
                                }));
                                ServerResponse::Ok
                            } else {
                                ServerResponse::Error("Unable to connect to server!".to_owned())
                            }
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
