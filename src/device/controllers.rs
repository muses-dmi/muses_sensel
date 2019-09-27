//! Description:
//! 
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 

use std::sync::mpsc::{Sender};
use std::str::FromStr;
use std::convert::From;
use std::time::{Duration, Instant};
use rosc::{OscPacket, OscMessage, OscType};
use rosc::encoder;
use std::cmp;



extern crate num;

use crate::sensel::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgType {
    IType(i32),
    FType(f32)
}

/// Convert from JSON argument type to OSC argument type
impl From<ArgType> for OscType {
    fn from(item: ArgType) -> Self {
        match item {
            ArgType::IType(i) => OscType::Int(i),
            ArgType::FType(f) => OscType::Float(f),
        }
    }
}

impl From<ArgType> for f32 {
    fn from(item: ArgType) -> Self {
        match item {
            ArgType::IType(i) => i as f32,
            ArgType::FType(f) => f,
        }
    }
}

/// All controllers implement this trait
pub trait Controller {
    /// the name of this controller
    fn name(&self) ->  &'static str; 

    /// process a touch event, outputs OSC
    /// messages to transport layer
    fn touch(&mut self, 
            contact: &contact::Contact, 
            transport: &Sender<OscPacket>)
                -> Result<(), &'static str>;
}

//-----------------------------------------------------------------------------

const TOUCH_START: i32 = 0;
const TOUCH_MOVE: i32  = 1;
const TOUCH_END: i32   = 2;

/// Pad controller
#[derive(Debug, Clone)]
pub struct Pad {
    /// OSC address
    address: String,
    /// static OSC message arguments
    args: Vec<OscType>,
    pressure: bool,
    generate_move: bool,
    generate_end: bool,
    generate_coords: bool,
    previous_time: Instant,
}

impl Pad {
    pub fn new(
            address: String, 
            args: Vec<ArgType>, 
            pressure: bool, 
            generate_move: bool,
            generate_end: bool,
            generate_coords: bool) -> Self {
        Pad {
            address: address,
            args: args.into_iter().map(|a| OscType::from(a)).collect(),
            pressure: pressure,
            generate_coords: generate_coords,
            generate_move: generate_move,
            generate_end: generate_end,
            previous_time: Instant::now(),
        }
    }
    //
}

impl Controller for Pad {
    fn name(&self) -> &'static str {
        "pad"
    }

    /// generate OSC message on start contact
    fn touch(&mut self, 
             contact: &contact::Contact, 
             transport: &Sender<OscPacket>)
                -> Result<(), &'static str> {
        
        //println!("{}", contact.total_force);
        if contact.total_force <= 20.0 {
            return Ok(());
        }

        if self.previous_time.elapsed() > Duration::from_millis(20) {
            match contact.state {
                contact::State::CONTACT_START => {
                    let args = 
                        if self.pressure && self.generate_coords {
                            let mut a = vec![OscType::Int(TOUCH_START),
                                            OscType::Float(contact.total_force), 
                                            OscType::Float(contact.x),
                                            OscType::Float(contact.x)];
                            a.extend(self.args.iter().cloned());
                            a
                        }
                        else if self.generate_coords {
                            let mut a = vec![
                                OscType::Int(TOUCH_START), 
                                OscType::Float(contact.x), 
                                OscType::Float(contact.y)];
                            a.extend(self.args.iter().cloned());
                            a
                        }
                        else if self.pressure {
                            let mut a = vec![OscType::Int(TOUCH_START), OscType::Float(contact.total_force)];
                            a.extend(self.args.iter().cloned());
                            a
                        }
                        else {
                            let mut a = vec![OscType::Int(TOUCH_START)];
                            a.extend(self.args.clone());
                            a
                        };

                    let packet = OscPacket::Message(OscMessage {
                        addr: self.address.clone(),
                        args: Some(args),
                    });
                    transport.send(packet).unwrap();

                    self.previous_time = Instant::now();
                },
                contact::State::CONTACT_MOVE => {
                    if self.generate_move {
                        let args = 
                            if self.pressure && self.generate_coords {
                                let mut a = vec![OscType::Int(TOUCH_MOVE),
                                                OscType::Float(contact.total_force), 
                                                OscType::Float(contact.x),
                                                OscType::Float(contact.x)];
                                a.extend(self.args.iter().cloned());
                                a
                            }
                            else if self.generate_coords {
                                let mut a = vec![
                                    OscType::Int(TOUCH_MOVE), 
                                    OscType::Float(contact.x), 
                                    OscType::Float(contact.y)];
                                a.extend(self.args.iter().cloned());
                                a
                            }
                            else if self.pressure {
                                let mut a = vec![OscType::Int(TOUCH_MOVE), OscType::Float(contact.total_force)];
                                a.extend(self.args.iter().cloned());
                                a
                            }
                            else {
                                let mut a = vec![OscType::Int(TOUCH_MOVE)];
                                a.extend(self.args.clone());
                                a
                            };

                        let packet = OscPacket::Message(OscMessage {
                            addr: self.address.clone(),
                            args: Some(args),
                        });
                        transport.send(packet).unwrap();
                    }
                },
                contact::State::CONTACT_END => {
                    self.previous_time = Instant::now();
                        // add if generate end
                        let args = 
                            if self.pressure && self.generate_coords {
                                let mut a = vec![OscType::Int(TOUCH_END),
                                                OscType::Float(contact.total_force), 
                                                OscType::Float(contact.x),
                                                OscType::Float(contact.x)];
                                a.extend(self.args.iter().cloned());
                                a
                            }
                            else if self.generate_coords {
                                let mut a = vec![OscType::Int(TOUCH_END), OscType::Float(contact.x), OscType::Float(contact.y)];
                                a.extend(self.args.iter().cloned());
                                a
                            }
                            else if self.pressure {
                                let mut a = vec![OscType::Int(TOUCH_END), OscType::Float(contact.total_force)];
                                a.extend(self.args.iter().cloned());
                                a
                            }
                            else {
                                let mut a = vec![OscType::Int(TOUCH_END)];
                                a.extend(self.args.clone());
                                a
                            };

                        let packet = OscPacket::Message(OscMessage {
                            addr: self.address.clone(),
                            args: Some(args),
                        });
                        transport.send(packet).unwrap();
                },
                // all other states are ignored
                _ => { }
            }
        }
        return Ok(());
    }
}

#[derive(Debug, Clone)]
pub struct VSlider {
    /// OSC address
    address: String,
    /// static OSC message arguments
    args: Vec<OscType>,
    /// minimum value for slider (default 0)
    min: f32,
    /// maximum value for slider (default 127)
    max: f32,
    /// increment for slider value
    incr: f32,
    /// current value of slider
    value: f32,
    /// last y position
    last_y: i32,
}

impl VSlider {
    pub fn new(
        address: String, args: Vec<ArgType>, 
        min: Option<ArgType>, max: Option<ArgType>,
        initial: Option<ArgType>, incr: Option<ArgType>) -> Self {
        VSlider {
            address: address,
            args: args.into_iter().map(|a| OscType::from(a)).collect(),
            min: min.map_or(0.0, |x| f32::from(x)),
            max: max.map_or(127.0, |x| f32::from(x)),
            incr: incr.map_or(1.0, |x| f32::from(x)),
            value: initial.map_or(0.0, |x| f32::from(x)),
            last_y: 0,
        }
    }
}

impl Controller for VSlider {
    fn name(&self) -> &'static str {
        "vslider"
    }

    fn touch(&mut self, 
             contact: &contact::Contact, 
             transport: &Sender<OscPacket>)
                -> Result<(), &'static str> {

        match contact.state {
            contact::State::CONTACT_START => {
                // set touch start position
                self.last_y = contact.y as i32;
                Ok(())
            },
            contact::State::CONTACT_END => {
                // reset on touch start and so nothing to do here
                Ok(())
            },
            contact::State::CONTACT_MOVE => {
                let y = contact.y as i32;

                // determine upwards or downwards movement (or no movement)
                let movement = (self.last_y - y) as f32 * self.incr;
                
                // update state to reflect current touch position
                self.last_y = y;
        
                // only send message if there was some movement
                if movement != 0.0 {
                    self.value = num::clamp(self.value + movement, self.min, self.max);
                    // build OSC argument list
                    let mut args = self.args.clone();
                    args.push(OscType::Float(self.value));
                    // create OSC packet and send
                    let packet = OscPacket::Message(OscMessage {
                        addr: self.address.clone(),
                        args: Some(args),
                    });
                    transport.send(packet).unwrap();
                }
                
                Ok(())    
            },
            _ => {
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HSlider {
    /// OSC address
    address: String,
    /// static OSC message arguments
    args: Vec<OscType>,
    /// minimum value for slider (default 0)
    min: f32,
    /// maximum value for slider (default 127)
    max: f32,
    /// increment for slider value
    incr: f32,
    /// current value of slider
    value: f32,
    /// last x position
    last_x: i32,
}

impl HSlider {
    pub fn new(
        address: String, args: Vec<ArgType>, 
        min: Option<ArgType>, max: Option<ArgType>,
        initial: Option<ArgType>, incr: Option<ArgType>) -> Self {
        HSlider {
            address: address,
            args: args.into_iter().map(|a| OscType::from(a)).collect(),
            min: min.map_or(0.0, |x| f32::from(x)),
            max: max.map_or(127.0, |x| f32::from(x)),
            incr: incr.map_or(1.0, |x| f32::from(x)),
            value: initial.map_or(0.0, |x| f32::from(x)),
            last_x: 0,
        }
    }
}

impl Controller for HSlider {
    fn name(&self) -> &'static str {
        "hslider"
    }

    fn touch(&mut self, 
             contact: &contact::Contact, 
             transport: &Sender<OscPacket>)
                -> Result<(), &'static str> {
               match contact.state {
            contact::State::CONTACT_START => {
                // set touch start position
                self.last_x = contact.y as i32;
                Ok(())
            },
            contact::State::CONTACT_END => {
                // reset on touch start and so nothing to do here
                Ok(())
            },
            contact::State::CONTACT_MOVE => {
                let x = contact.x as i32;
                
                // determine left or right movement (or no movement)
                let movement = (self.last_x - x) as f32 * self.incr;

                // update state to reflect current touch position
                self.last_x = x;
                
                // only send message if there was some movement
                if movement != 0.0 {
                    self.value = num::clamp(self.value + movement, self.min, self.max);
                    // build OSC argument list
                    let mut args = self.args.clone();
                    args.push(OscType::Float(self.value));
                    // create OSC packet and send
                    let packet = OscPacket::Message(OscMessage {
                        addr: self.address.clone(),
                        args: Some(args),
                    });
                    transport.send(packet).unwrap();
                }
                
                Ok(())    
            },
            _ => {
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Endless {
    /// OSC address
    address: String,
    /// static OSC message arguments
    args: Vec<OscType>,
}

impl Endless {
    pub fn new(address: String, args: Vec<ArgType>) -> Self {
        Endless {
            address: address,
            args: args.into_iter().map(|a| OscType::from(a)).collect(),
        }
    }
}

impl Controller for Endless {
    fn name(&self) -> &'static str {
        "endless"
    }

    fn touch(&mut self, 
             contact: &contact::Contact, 
             transport: &Sender<OscPacket>)
                -> Result<(), &'static str> {
        Ok(())
    }
}