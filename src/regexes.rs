use regex::{
    Regex,
};

lazy_static!{
    //CLI command pieces
    pub static ref connect: Regex =         Regex::new(r"^connect\s+(\S+)").unwrap();
    pub static ref disconnect: Regex =      Regex::new(r"^disconnect").unwrap();
    pub static ref rawcmd: Regex =          Regex::new(r"^r\s+(\S+)").unwrap();

    pub static ref radio: Regex =           Regex::new(r"^radio\s+(\S+)").unwrap();

    pub static ref on: Regex =              Regex::new(r"^on").unwrap();
    pub static ref off: Regex =             Regex::new(r"^off").unwrap();
    pub static ref set: Regex =             Rexex::new(r"^set\s+(\S+)").unwrap();
}
