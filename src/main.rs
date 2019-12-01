//
// Program to display a clock on the ZeroSeg display board mounted on a Raspberry Pi Zero W.
//
use std::time::Duration;
use std::thread::{sleep, spawn};
use std::sync::mpsc::{channel, Sender};
use chrono::prelude::*;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use rppal::gpio::{Gpio, Level, Trigger};

fn main() {
    // Open the SPI bus to communicate with the attached MAX7219CNG display driver.
    let mut display = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 10_000_000, Mode::Mode0)
        .expect("Failed to create Spi object.");
    
    // Create a messaging channel for the non-blocking write to SPI bus.  This is
    //  created so that the sending of data to the display can be queued and so
    //  is effectively non-blocking for the main process.
    let (display_tx, display_rx) = channel();

    // Spawn a process to accept display commands and dispatch
    // them to the SPI interface.
    spawn(move || {
        for (command, value) in display_rx {
            let buffer = [command, value];
            display.write(&buffer)
                .expect("Failed to write to SPI.");
        }
    });

    // Initialise the display device.
    // Sets parameters for the MAX7219CNG and initialises display options for
    //  the program.
    let mut display_inverted = true;
    let mut display_intensity = 7; 
    display_tx.send((0x9, 0x00)).unwrap(); // Disable decode mode for all digits.
    display_tx.send((0xA, display_intensity)).unwrap();  // Set intensity.
    display_tx.send((0xB, 0x7)).unwrap();  // Set scan-limit to 0-7.
    display_tx.send((0xC, 0x1)).unwrap();  // Set normal operation (not shut-domn).
    display_tx.send((0xF, 0x0)).unwrap();  // Set test mode off.



    // Create a messaging channel for main event handler and clone it for each sender.
    let (main_tx, main_rx) = channel();             // 1 second "interrupt".
    let main_tx_switch1 = Sender::clone(&main_tx);  // Switch 1 event.
    let main_tx_switch2 = Sender::clone(&main_tx);  // Switch 2 event.


    // Initialise buttons.
    // Buttons will send a message to the main routine when the appropriate
    //  interrupt is detected.
    let mut switch1 = Gpio::new().unwrap().get(17).unwrap().into_input();
    switch1.set_async_interrupt(Trigger::FallingEdge, move |level| {
        main_tx_switch1.send(MainMessage::ButtonChange(1, level)).unwrap();
        }).unwrap();
 
    let mut switch2 = Gpio::new().unwrap().get(26).unwrap().into_input();
    switch2.set_async_interrupt(Trigger::FallingEdge, move |level| {
        main_tx_switch2.send(MainMessage::ButtonChange(2, level)).unwrap();
        }).unwrap();
     

    // Initiate the 1 second "interrupt" routine.  This is the "pendulum"
    //  of the program.  It sends a message to the main routine whenever a
    //  second completes.
    spawn(move || {
        loop {
            let dt = Local::now();
            let time_to_sleep = Duration::new(0, 1_000_000_000 - dt.nanosecond());
            sleep(time_to_sleep);
            main_tx.send(MainMessage::TimeSignal).unwrap();
        }
    });

    for main_message in main_rx {
        match main_message {
            MainMessage::TimeSignal => {
                disp_time(&display_tx, display_inverted);
            }
            MainMessage::ButtonChange(switch, _level) => {
                // println!("Switch {} went {}", switch, level);
                if switch == 1 {
                    if display_inverted {
                        display_inverted = false;
                    } else {
                        display_inverted = true;
                    }
                    disp_time(&display_tx, display_inverted);
                } else {
                    display_intensity = (display_intensity + 1) % 16;
                    display_tx.send((0xA, display_intensity)).unwrap();  // Set intensity.
                }

            } 
        }
    }
}

// Function to display the current local time in hh-mm-ss format.  The digits are displayed by
//  sending commands to the specified output queue.  The display can be inverted to deal with
//  its orientation when installed.
fn disp_time(display_tx: &std::sync::mpsc::Sender<(u8, u8)>,
        display_inverted: bool) {
    let dt = Local::now();
    let hour_high: u8 = (dt.hour() / 10) as u8;
    let hour_low: u8 = (dt.hour() % 10) as u8;
    let minute_high: u8 = (dt.minute() /10) as u8;
    let minute_low: u8 = (dt.minute() % 10) as u8;
    let second_high: u8 = (dt.second() / 10) as u8;
    let second_low: u8 = (dt.second() % 10) as u8;        
    if display_inverted {
        display_tx.send((0x8, decode_digit(second_low, DigitOrientation::Inverted, false))).unwrap();
        display_tx.send((0x7, decode_digit(second_high, DigitOrientation::Inverted, false))).unwrap();
        display_tx.send((0x6, 0x1)).unwrap();
        display_tx.send((0x5, decode_digit(minute_low, DigitOrientation::Inverted, false))).unwrap();
        display_tx.send((0x4, decode_digit(minute_high, DigitOrientation::Inverted, false))).unwrap();
        display_tx.send((0x3, 0x1)).unwrap();
        display_tx.send((0x2, decode_digit(hour_low, DigitOrientation::Inverted, false))).unwrap();
        display_tx.send((0x1, decode_digit(hour_high, DigitOrientation::Inverted, false))).unwrap();
    } else {
        display_tx.send((0x1, decode_digit(second_low, DigitOrientation::Normal, false))).unwrap();
        display_tx.send((0x2, decode_digit(second_high, DigitOrientation::Normal, false))).unwrap();
        display_tx.send((0x3, 0x1)).unwrap();
        display_tx.send((0x4, decode_digit(minute_low, DigitOrientation::Normal, false))).unwrap();
        display_tx.send((0x5, decode_digit(minute_high, DigitOrientation::Normal, false))).unwrap();
        display_tx.send((0x6, 0x1)).unwrap();
        display_tx.send((0x7, decode_digit(hour_low, DigitOrientation::Normal, false))).unwrap();
        display_tx.send((0x8, decode_digit(hour_high, DigitOrientation::Normal, false))).unwrap();
    }
}

// Function to convert a decimal digit from a u8 specifying its absolute value, to a u8 bitmap
//  indicating the segments to be lit for passing to a seven-segment display.  The returned
//  value can also be "inverted" depending on required display orientation.
fn decode_digit(digit: u8, orientation: DigitOrientation, dp: bool) -> u8 {
    let mut return_value = 0;
    match orientation {
        DigitOrientation::Normal => {
            match digit {
                0 => {
                    return_value += 126;
                }
                1 => {
                    return_value += 48;
                }
                2 => {
                    return_value += 109;
                }
                3 => {
                    return_value += 121;
                }
                4 => {
                    return_value += 51;
                }
                5 => {
                    return_value += 91;
                }
                6 => {
                    return_value += 95;
                }
                7 => {
                    return_value += 112;
                }
                8 => {
                    return_value += 127;
                }
                9 => {
                    return_value += 123;
                }
                _ => {
                    return_value += 0;
                }
            }
        }
        DigitOrientation::Inverted => {
            match digit {
                0 => {
                    return_value += 126;
                }
                1 => {
                    return_value += 6;
                }
                2 => {
                    return_value += 109;
                }
                3 => {
                    return_value += 79;
                }
                4 => {
                    return_value += 23;
                }
                5 => {
                    return_value += 91;
                }
                6 => {
                    return_value += 123;
                }
                7 => {
                    return_value += 14;
                }
                8 => {
                    return_value += 127;
                }
                9 => {
                    return_value += 95;
                }
                _ => {
                    return_value += 0;
                }
            }
        }
    }
    if dp {
        return_value += 1;
    }
    return_value
}

enum DigitOrientation {
    Normal,
    Inverted,
}


enum MainMessage {
    TimeSignal,
    ButtonChange(u8, Level),
}
