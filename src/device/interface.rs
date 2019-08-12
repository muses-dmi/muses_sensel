//! Description:
//! 
//! 
//! Copyright © 2019 Benedict Gaster. All rights reserved.
//! 

use super::controllers::*;

use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::{Value};

use std::str::FromStr;
use std::time::Duration;
use std::sync::mpsc::{Sender};

use std::{thread, time};

use rosc::{OscPacket, OscMessage, OscType};

use crate::sensel;

use std::io::stdin;
use std::thread::spawn;
use std::sync::mpsc::channel;

use crate::sensel::device::Device;

//-----------------------------------------------------------------------------
// constants

const TYPE_PAD : &'static str = "pad";
const TYPE_HSLIDER : &'static str = "horz_slider";
const TYPE_VSLIDER : &'static str = "vert_slider"; 
const TYPE_ENDLESS : &'static str = "endless";

const NONE_ID: ID = 0;

//-----------------------------------------------------------------------------

type ID = u32;

pub struct Interface {
    buffer: Vec<Vec<ID>>,
    controls: Vec<Box<Controller>>,
    device: sensel::device::BaseDevice,
}

impl Interface {
    pub fn new(
        buffer: Vec<Vec<ID>>, 
        controls: Vec<Box<Controller>>, 
        device: sensel::device::BaseDevice) -> Self {
        Interface {
            buffer: buffer,
            controls: controls,
            device: device,
        }
    }

    /// Process Morph data, returns only on exit
    pub fn run(mut self, hetz: u32, transport: Sender<OscPacket>) {
        //let d: Box<Device> = Box::new(self.device._get_device());

        self.device.set_frame_content(sensel::frame::Mask::CONTACTS).unwrap();

        let scan = self.device.start_scanning().unwrap();

        println!("Press Enter to exit driver");
        let (sender, receiver) = channel();
        spawn(move || {
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            sender.send(()).unwrap();
        });

        // target duration of a single, i.e. run at the speed specified by caller
        let frame_duration_ms = time::Duration::from_millis((1000.0 / hetz as f32) as u64);

        while receiver.try_recv().is_err() {
            // read current time
            let now = time::Instant::now();

            // read sensor image
            scan.read_sensor().unwrap();
            let num_frames = scan.get_num_available_frames().unwrap();

            for _ in 0..num_frames {
                let frame = scan.get_frame().unwrap();
                let contacts = frame.contacts.unwrap();

                if contacts.len() > 0 {
                    info!("Num Contacts: {}", contacts.len());
                    for &contact in contacts {
                        let contact = sensel::contact::Contact::from(contact);
                        info!(
                            "Contact ID: {} State: {:?} @Location({},{})", 
                            contact.id, contact.state, contact.x, contact.y);
                        //println!("Buffer ID: {}", self.buffer[contact.x as usize][contact.y as usize]);

                        if self.buffer[contact.x as usize][contact.y as usize] != NONE_ID {
                            info!("Hit({}) {} [{},{}]", 
                                self.buffer[contact.x as usize][contact.y as usize],
                                self.controls[self.buffer[contact.x as usize][contact.y as usize] as usize - 1].name(),
                                contact.x, contact.y);

                            match self.controls[self.buffer[contact.x as usize][contact.y as usize] as usize - 1].touch(
                                &contact,
                                &transport) {
                                Ok(_) => {},
                                Err(s) => error!("{}", s)
                            };
                            // match contact.state {
                            //     sensel::contact::State::CONTACT_START => {
                            //         scan.device().set_led_brightness(contact.id, 100).unwrap();
                            //     },
                            //     sensel::contact::State::CONTACT_END => {
                            //         scan.device().set_led_brightness(contact.id, 0).unwrap();
                            //     },
                            //     _ => {}
                            // };
                        }
                        else {
                            transport.send(OscPacket::Message(OscMessage {
                                        addr: "/not/a/msg".to_string(),
                                        args: None,
                                })).unwrap();   
                        }

                        // wait for any remaining time before processsing next frame
                        info!("Frame duration was {:?}", now.elapsed());
                        // let et = frame_duration_ms - now.elapsed();

                        // if et > Duration::from_millis(0) {
                        //     thread::sleep(et);
                        // }
                    }
                    //println!();
                }
            }
        }

    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Control {
    pub address: String,
    pub args: Vec<ArgType>,
    pub id: usize,
    pub rgb:  Option<String>,
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

    /// connect to sensel if one present otherwise return error
    fn sensel_info(&self) -> Result<sensel::device::BaseDevice, &'static str> {
        let mut input = String::new();

        let list = sensel::device::get_device_list().unwrap();

        let list_slice = list.as_slice();

        if list_slice.len() == 0 {
            return Err("no Sensel device found");
        }

        let device_id = list_slice[0];
        let device = device_id.open().unwrap();

        info!("Sensel Device: {}" , device_id.get_serial_num() );
        info!("COM port: {}" , device_id.get_com_port() );
        info!("Firmware Version: {}.{}.{}", 
            device.info.fw_info.fw_version_major, 
            device.info.fw_info.fw_version_minor, 
            device.info.fw_info.fw_version_build);
        info!("Width: {}mm", device.info.sensor_info.width);
        info!("Height: {}mm", device.info.sensor_info.height);
        info!("Cols: {}", device.info.sensor_info.num_cols);
        info!("Rows: {}", device.info.sensor_info.num_rows);

        Ok(device)
    }

    fn connect(&self) -> Result<sensel::device::BaseDevice, &'static str> {
        self.sensel_info()
    }

    /// build interface
    /// Loads JSON IR interface
    /// Connects to Sensel Morph
    /// In the case that either of these tasks fail return an error, otherwise a valid interface
    pub fn build(&self) -> Result<Interface, &'static str> {
        self.connect().and_then(|d| self.build_internal(d))
    }

    pub fn build_internal(&self, device: sensel::device::BaseDevice) -> Result<Interface, &'static str> {

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
                                    let pad = Box::new(Pad::new(ctl.address, ctl.args));
                                    info!("adding pad = {}", ctl.id);
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

                            Ok(Interface::new(buffer, controls, device))
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