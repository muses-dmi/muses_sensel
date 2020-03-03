//! Description:
//! 
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 

use std::net::{SocketAddrV4};

use super::controllers::*;

use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::{Value};

use std::str::FromStr;
use std::time::Duration;
use std::sync::mpsc::{Sender};

use std::{thread, time};

use rosc::{OscPacket, OscMessage, OscType};

use std::sync::atomic::{AtomicBool, Ordering};

use crate::sensel;

use std::io::stdin;
use std::thread::spawn;
use std::sync::mpsc::channel;

use crate::sensel::*;

use crate::sensel::device::Device;

//-----------------------------------------------------------------------------
// constants

const TYPE_PAD : &'static str = "pad";
const TYPE_DPAD : &'static str = "dpad";
const TYPE_HSLIDER : &'static str = "horz_slider";
const TYPE_VSLIDER : &'static str = "vert_slider"; 
const TYPE_ENDLESS : &'static str = "endless";

const NONE_ID: ID = 0;

//-----------------------------------------------------------------------------

const MAX_NUM_IDS: usize = 16;

type ID = u32;

pub struct InterfaceDirect {
    buffer: Vec<Vec<ID>>,
    controls: Vec<Box<Controller>>,
    move_end: [Option<ID>; MAX_NUM_IDS],
}

impl InterfaceDirect {
    pub fn new(
        buffer: Vec<Vec<ID>>, 
        controls: Vec<Box<Controller>>) -> Self {
        InterfaceDirect {
            buffer: buffer,
            controls: controls,
            move_end: [None; MAX_NUM_IDS],
        }
    }

    /// process contact from external (sensel) interface 
    pub fn handleContact(
        &mut self,
        contact: &contact::Contact, 
        transport: &Sender<(OscPacket, Option<SocketAddrV4>)>) {
            match contact.state {
                sensel::contact::State::CONTACT_START => {
                    let id = self.buffer[contact.x as usize][contact.y as usize];
                    if id != NONE_ID {
                        match self.controls[id as usize - 1].touch_start(&contact, &transport) {
                            Ok(_) => {},
                            Err(s) => error!("{}", s)
                        };

                        // Store ID is can produce end touch event for contact even if it has left control area itself
                        
                        self.move_end[contact.id as usize] = Some(id);
                    }
                },
                sensel::contact::State::CONTACT_MOVE => {

                },
                sensel::contact::State::CONTACT_END => {
                    if let Some(id) = self.move_end[contact.id as usize] {
                        self.controls[id as usize - 1].touch_end(&contact, &transport);
                        self.move_end[contact.id as usize] = None;
                    }
                },
                _ => { 
                },
            }
    }
}

    /// Process Morph data, returns only on exit
    // pub fn run(mut self, hetz: u32, transport: Sender<OscPacket>, disconnect: &AtomicBool) {
    //     //let d: Box<Device> = Box::new(self.device._get_device());

    //     self.device.set_frame_content(sensel::frame::Mask::CONTACTS).unwrap();

    //     let scan = self.device.start_scanning().unwrap();

    //     // target duration of a single, i.e. run at the speed specified by caller
    //     let frame_duration_ms = time::Duration::from_millis((1000.0 / hetz as f32) as u64);

    //     while !disconnect.load(Ordering::SeqCst) {
    //         // read current time
    //         let now = time::Instant::now();

    //         // read sensor image
    //         scan.read_sensor().unwrap();
    //         let num_frames = scan.get_num_available_frames().unwrap();

    //         for _ in 0..num_frames {
    //             let frame = scan.get_frame().unwrap();
    //             let contacts = frame.contacts.unwrap();

    //             if contacts.len() > 0 {
    //                 info!("Num Contacts: {}", contacts.len());
    //                 for &contact in contacts {
    //                     let contact = sensel::contact::Contact::from(contact);
    //                     info!(
    //                         "Contact ID: {} State: {:?} @Location({},{})", 
    //                         contact.id, contact.state, contact.x, contact.y);
    //                     //println!("Buffer ID: {}", self.buffer[contact.x as usize][contact.y as usize]);

    //                     if self.buffer[contact.x as usize][contact.y as usize] != NONE_ID {
    //                         info!("Hit({}) {} [{},{}]", 
    //                             self.buffer[contact.x as usize][contact.y as usize],
    //                             self.controls[self.buffer[contact.x as usize][contact.y as usize] as usize - 1].name(),
    //                             contact.x, contact.y);

    //                         match self.controls[self.buffer[contact.x as usize][contact.y as usize] as usize - 1].touch(
    //                             &contact,
    //                             &transport) {
    //                             Ok(_) => {},
    //                             Err(s) => error!("{}", s)
    //                         };
    //                         match contact.state {
    //                             sensel::contact::State::CONTACT_START => {
    //                                 //scan.device().set_led_brightness(contact.id, 100).unwrap();
    //                             },
    //                             sensel::contact::State::CONTACT_END => {
    //                                 //scan.device().set_led_brightness(contact.id, 0).unwrap();
    //                             },
    //                             _ => {}
    //                         };
    //                     }
    //                     else {
    //                         transport.send(OscPacket::Message(OscMessage {
    //                                     addr: "/not/a/msg".to_string(),
    //                                     args: None,
    //                             })).unwrap();   
    //                     }

    //                     // wait for any remaining time before processsing next frame
    //                     info!("Frame duration was {:?}", now.elapsed());
    //                     // let et = frame_duration_ms - now.elapsed();

    //                     // if et > Duration::from_millis(0) {
    //                     //     thread::sleep(et);
    //                     // }
    //                 }
    //                 //println!();
    //             }
    //         }
    //     }

    // }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Control {
    pub address: String,
    pub args: Vec<ArgType>,
    pub id: usize,
    pub rgb:  Option<String>,
    pub pressure: Option<bool>,
    pub generate_move: Option<bool>,
    pub generate_end: Option<bool>,
    pub generate_coords: Option<bool>,
    pub on: Option<ArgType>,
    pub off: Option<ArgType>,
    pub type_id: String,
    pub min: Option<ArgType>,
    pub max: Option<ArgType>,
    pub initial: Option<ArgType>, 
    pub incr: Option<ArgType>,
}

pub struct InterfaceBuilder {
    input: String,
    number_of_controllers: u32,
}

impl InterfaceBuilder {
    pub fn new (input: String) -> Self {
        InterfaceBuilder {
            input: input,
            number_of_controllers: 0,
        }
    }

    /// build interface
    /// Loads JSON IR interface
    /// Connects to Sensel Morph
    /// In the case that either of these tasks fail return an error, otherwise a valid interface
    pub fn build(&self) -> Result<InterfaceDirect, &'static str> {
        self.build_internal()
    }

    pub fn build_internal(&self) -> Result<InterfaceDirect, &'static str> {

        let v : serde_json::Result<Value>  = serde_json::from_str(&self.input);
        match v {
            Ok(Value::Object(obj)) => {
                //let interface = Interface::new();

                if obj.contains_key("buffer") {
                    let buffer: Vec<Vec<u32>> =  serde_json::from_value(obj["buffer"].clone()).unwrap();
                    //println!("{} {}", buffer.len(), buffer[0].len());
                    match obj.get("controllers") {
                        Some(Value::Array(controllers)) => {
                            let number_of_controllers = controllers.len();

                            // create an ordered (on ID) list of controls
                            let mut cs: Vec<Control> = vec![];
                            for c in controllers {
                                // TODO: add check for Error and return an error is so
                                let ctl: Control = serde_json::from_value(c.clone()).expect("unxpected format error with controller");
                                cs.push(ctl.clone());
                            }
                            cs.sort_by(|a, b| a.id.cmp(&b.id));
                            
                            // Each controller has a unique ID, between 0..number_of_controllers-1, which is 
                            // used as a direct index into array of Controller instances
                            let mut controls: Vec<Box<Controller>> = vec![];

                            for ctl in cs {
                                if ctl.type_id == TYPE_PAD {
                                    let pressure = ctl.pressure.map_or(false, |_| true);
                                    let generate_move = ctl.generate_move.map_or(false, |_| true);
                                    let generate_end = ctl.generate_end.map_or(false, |_| true);
                                    let generate_coords = ctl.generate_coords.map_or(false, |_| true);
                                    
                                    let pad = Box::new(
                                        Pad::new(
                                            ctl.address, 
                                            ctl.args, 
                                            pressure, 
                                            generate_move, 
                                            generate_end,
                                            generate_coords));
                                    info!("adding pad = {}", ctl.id);
                                    controls.push(pad);
                                }
                                else if ctl.type_id == TYPE_DPAD {
                                    let on = ctl.on.map_or(ArgType::IType(0), |x| x);
                                    let off = ctl.off.map_or(ArgType::IType(0), |x| x);
                                    
                                    let pad = Box::new(
                                        DPad::new(
                                            ctl.address, 
                                            on,
                                            off,
                                            ctl.args));
                                    info!("adding dpad = {}", ctl.id);
                                    controls.push(pad);
                                }
                                else if ctl.type_id == TYPE_VSLIDER { 
                                    let vslider = Box::new(
                                        VSlider::new(ctl.address, ctl.args, ctl.min, ctl.max, ctl.initial, ctl.incr));
                                    info!("adding vslider = {}", ctl.id);
                                    controls.push(vslider);
                                }
                                else if ctl.type_id == TYPE_HSLIDER {
                                    let hslider = Box::new(
                                        HSlider::new(ctl.address, ctl.args, ctl.min, ctl.max, ctl.initial, ctl.incr));
                                    info!("hslider = {}", ctl.id);
                                    controls.push(hslider);

                                }
                                else if ctl.type_id == TYPE_ENDLESS {
                                    let endless = Box::new(Endless::new(ctl.address, ctl.args));
                                    info!("endless = {}", ctl.id);
                                    controls.push(endless);
                                }
                            }

                            Ok(InterfaceDirect::new(buffer, controls))
                        },
                        _ => Err("failed to find controllers array"), 
                    }
                }
                else {
                    Err("failed to find buffer")
                }
            },
            _ => Err("failed to pass JSON IR")
        }
    } 
}