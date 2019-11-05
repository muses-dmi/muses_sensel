//! Description:
//! 
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate bitflags;

use std::{env};

use std::net::{UdpSocket, SocketAddrV4};
use std::str::FromStr;
use std::time::Duration;

pub mod sensel;
pub use sensel::*;

pub mod device;
pub use device::*;