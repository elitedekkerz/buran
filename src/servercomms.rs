use regex::{
    Regex,
    Captures,
};
use std::net::{
    SocketAddr,
    TcpStream,
    Shutdown,
};
use std::io::{
    Read,
    Write,
};

#[derive(Debug)]
pub struct RadarObject {
    range: f64,
    elevation: f64,
    azimuth: f64,
}

#[derive(Debug)]
pub struct CrewMember {
    name: String,
}

#[derive(Debug)]
pub enum ServerResponse {
    Error(String),
    Ok,

    GeneratorGet(f64, f64),

    RadarScan(Vec<RadarObject>),
    RadarSector(f64),
    
    Echo(String),
    
    LogRead(String),

    Crew(Vec<CrewMember>),

    Ship{
        position: Option<(f64,f64,f64)>,
        velocity: Option<(f64,f64,f64)>,
        heading: Option<(f64,f64,f64)>,
        power: Option<(f64,f64)>,
    },

    Radio(bool),

    Time(f64),

    RawResponse(String),
}

#[derive(Debug)]
pub enum ServerCommand<'a> {
    Echo(&'a str),

    GeneratorGet,
    GeneratorSet(f64),

    RadarScan,
    RadarSector,
    RadarOn(bool),
    RadarIdentify(&'a str),

    CrewList,

    ShipPosition,
    ShipVelocity,
    ShipHeading,
    ShipPower,

    RadioOn(bool),
    RadioSet(f64),
    RadioGet,

    Time,

    Rudder(Option<f64>, Option<f64>, Option<f64>),

    LogWrite(&'a str),
    LogRead,
    LogClear,

    Thruster(Option<f64>, Option<f64>),

    RawCommand(&'a str),
}

pub trait ServerCommunicator {
    fn server_command(&mut self, cmd: &ServerCommand) -> ServerResponse;
    fn disconnect(&mut self) -> Result<(),()>;
}

pub struct CmdlineCommunicator {
    pub tcpstream: TcpStream,
}

impl ServerCommunicator for CmdlineCommunicator {
    fn server_command(&mut self, cmd: &ServerCommand) -> ServerResponse {
        lazy_static!{
            static ref error_regex: Regex =         Regex::new(r"\nError\n").unwrap();
            static ref ok_regex: Regex =            Regex::new(r"\nOk\n(.+)").unwrap();

            static ref generator_regex: Regex =     Regex::new(r"Reactor is set to (\d+.\d+) and generates (\d+.\d+) kW of power.").unwrap();

            static ref radio_off_regex: Regex =     Regex::new(r"the radio is off").unwrap();

        };

        let mut command: String = match cmd {
            &ServerCommand::RawCommand(text) => {
                String::from(text)
            },
            _ => {
                println!("command {:?} not implemented!", cmd);
                String::new()
            }
        };
        command.push('\n');
        self.tcpstream.write(&command.as_bytes()).unwrap();
        self.tcpstream.flush().unwrap();
        
        static prompt: &'static str = "Yuri@Восток:"; //TODO: the prompt can change
        let mut response_vec = Vec::new();

        while !response_vec.ends_with(prompt.as_bytes()) {
            let mut buf = [0u8];
            let read = self.tcpstream.read(&mut buf);
            if let Ok(1) = read {
                response_vec.push(buf[0]);
            }
        }

        let mut response = String::from_utf8(response_vec).expect("Server response not valid UTF-8!");
        let response_length = response.len()-prompt.len();
        response.truncate(response_length);

        //TODO: not implemented yet in server side
        //if error_regex.is_match(response) { return ServerResponse::Error(format!("{:?} command error", self.last_command)); }
        //let response = ok_regex.captures(response)
        //    .expect(format!("Server sent invalid response (not Error nor Ok!):\n{:?}", response))
        //    .get(1);

        match cmd {
            &ServerCommand::Echo(_) => ServerResponse::Echo(response),
            &ServerCommand::LogWrite(_) => ServerResponse::Ok,
            &ServerCommand::LogRead => ServerResponse::LogRead(response),
            &ServerCommand::GeneratorGet => {
                let captures = generator_regex.captures(&response)
                               .expect(&format!("Server sent invalid generator response: {:?}", response));
                ServerResponse::GeneratorGet(
                    captures.get(1).unwrap().as_str().parse()
                            .expect("Server sent invalid float as generator factor!"),
                    captures.get(2).unwrap().as_str().parse()
                            .expect("Server send invalid float as generator output!"))
            },
            &ServerCommand::RawCommand(_) => ServerResponse::RawResponse(response),
            &ServerCommand::RadioOn(_) => ServerResponse::Ok,
            &ServerCommand::RadioSet(_) => ServerResponse::Ok,
            &ServerCommand::RadioGet => ServerResponse::Radio(!radio_off_regex.is_match(&response)),
            _ => ServerResponse::Ok
        }
    }

    fn disconnect(&mut self) -> Result<(),()> {
        self.tcpstream.shutdown(Shutdown::Both).map_err(|_|())?;
        Ok(())
    }
}
