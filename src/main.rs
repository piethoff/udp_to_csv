use std::path::PathBuf; 
use std::net::UdpSocket;
use std::time::Duration;
use clap::{Parser, ValueEnum};
use std::fmt;

use local_ip_address::list_afinet_netifas;

use std::thread;
use std::sync::mpsc::{self, Receiver};

use std::io::Cursor;
use byteorder::{NetworkEndian, ReadBytesExt};

use std::fs::OpenOptions;
use std::io::prelude::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Address of local interface
    #[arg(long, short)]
    bind: std::net::IpAddr,

    /// Local port
    #[arg(long, short)]
    port: u16,

    /// data type of values
    #[arg(value_enum, short, long, default_value_t = DataType::U16)]
    data_type: DataType,

    /// csv file to write, if not given print to stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Clone, ValueEnum)]
enum DataType {
    Bool,
    U8,
    U16,
    I8,
    I16,
}
impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            DataType::Bool => "bool",
            DataType::U8 =>  "u8",
            DataType::U16 => "u16",
            DataType::I8 =>  "i8",
            DataType::I16 => "i16",
        })
    }
}
impl std::str::FromStr for DataType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "BOOLEAN" => Ok(DataType::Bool),
            "BOOL" =>    Ok(DataType::Bool),
            "U8" =>  Ok(DataType::U8),
            "U16" => Ok(DataType::U16),
            "I8" =>  Ok(DataType::I8),
            "I16" => Ok(DataType::I16),
            _ => Err("invalid datatype"),
        }
    }
}

fn print_local_interfaces() {
    let network_interfaces = list_afinet_netifas();

    if let Ok(network_interfaces) = network_interfaces {
        for (name, ip) in network_interfaces.iter() {
            println!("\t{}:\t{:?}", name, ip);
        }
    } else {
        println!("Error getting network interfaces: {:?}", network_interfaces);
    }
}

fn main() {
    let cli = Cli::parse();

    let socket = UdpSocket::bind((cli.bind, cli.port));
    if let Err(e) = socket {
        eprintln!("Could not bind to provided address {}:{}; {}", cli.bind, cli.port, e);
        println!("Avaliable network interfaces: ");
        print_local_interfaces();
        return;
    }
    let socket = socket.unwrap();
    socket.set_read_timeout(None).expect("set_read_timeout call failed");

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        writer(rx, cli);
    });

    let mut buffer = [0u8; 512];
    loop {
        let recv_result = socket.recv(&mut buffer);
        match recv_result {
            Err(e) => { eprintln!("Error receiving message: {e}"); },
            Ok(len) => { tx.send(buffer[0..len].to_vec()).expect("writer thread disconnected"); },
        };
    }
}

fn writer(rx: Receiver<Vec<u8>>, options: Cli) {
    let mut csv_string = "".to_owned();
    let mut count = 0;
    loop {
        let recv_result = rx.try_recv();
        match recv_result {
            Err(mpsc::TryRecvError::Empty) => thread::sleep(Duration::from_millis(50)),
            Err(mpsc::TryRecvError::Disconnected) => {
                match &options.output {
                    None => print!("{csv_string}"),
                    Some(output) => output_csv(&csv_string, output),
                }
                eprintln!("recv thread disconnected");
                return;
            },
            Ok(message) => {
                let mut cursor = Cursor::new(message);
                match options.data_type {
                    DataType::Bool => {
                        'read: loop {
                            match cursor.read_u8() {
                                Ok(value) => {
                                    for i in 0..8 {
                                        let value_bit = value >> i & 1;
                                        csv_string.push_str(&value_bit.to_string());
                                        csv_string.push(',');
                                    }
                                },
                                Err(e) => {
                                    match e.kind() {
                                        std::io::ErrorKind::UnexpectedEof => break 'read,
                                        _ => {
                                            eprintln!("error while parsing: {e}");
                                            break 'read;
                                        }
                                    };
                                },
                            };
                        }
                    },
                    DataType::U8 => { 
                        'read: loop {
                            match cursor.read_u8() {
                                Ok(value) => {
                                    csv_string.push_str(&value.to_string());
                                    csv_string.push(',');
                                },
                                Err(e) => {
                                    match e.kind() {
                                        std::io::ErrorKind::UnexpectedEof => break 'read,
                                        _ => {
                                            eprintln!("error while parsing: {e}");
                                            break 'read;
                                        }
                                    };
                                },
                            };
                        }
                    },
                    DataType::U16 => { 
                        'read: loop {
                            match cursor.read_u16::<NetworkEndian>() {
                                Ok(value) => {
                                    csv_string.push_str(&value.to_string());
                                    csv_string.push(',');
                                },
                                Err(e) => {
                                    match e.kind() {
                                        std::io::ErrorKind::UnexpectedEof => break 'read,
                                        _ => {
                                            eprintln!("error while parsing: {e}");
                                            break 'read;
                                        }
                                    };
                                },
                            };
                        }
                    },
                    DataType::I8 => { 
                        'read: loop {
                            match cursor.read_i8() {
                                Ok(value) => {
                                    csv_string.push_str(&value.to_string());
                                    csv_string.push(',');
                                },
                                Err(e) => {
                                    match e.kind() {
                                        std::io::ErrorKind::UnexpectedEof => break 'read,
                                        _ => {
                                            eprintln!("error while parsing: {e}");
                                            break 'read;
                                        }
                                    };
                                },
                            };
                        }
                    },
                    DataType::I16 => { 
                        'read: loop {
                            match cursor.read_i16::<NetworkEndian>() {
                                Ok(value) => {
                                    csv_string.push_str(&value.to_string());
                                    csv_string.push(',');
                                },
                                Err(e) => {
                                    match e.kind() {
                                        std::io::ErrorKind::UnexpectedEof => break 'read,
                                        _ => {
                                            eprintln!("error while parsing: {e}");
                                            break 'read;
                                        }
                                    };
                                },
                            };
                        }
                    },
                };

                // csv_string.replace_range((csv_string.len()-1)..csv_string.len(), "\n");
                let _ = csv_string.pop();

                match &options.output {
                    None => {
                        print!("{csv_string}");
                        csv_string.clear();
                    },
                    Some(output) => {
                        count += 1;
                        if count >= 1000 {
                            output_csv(&csv_string, output);
                            csv_string.clear();
                            count = 0;
                        }
                    },
                }
            },
        };
    }
}

fn output_csv(csv_string: &str, output: &PathBuf) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(output)
        .unwrap();

    if let Err(e) = writeln!(file, "{csv_string}") {
        eprintln!("Couldn't write to file: {}", e);
    }
}
