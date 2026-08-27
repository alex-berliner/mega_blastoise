#[cfg(feature = "usb")]
pub mod usb;

#[cfg(feature = "buzzer")]
pub mod buzzer;

#[cfg(feature = "oled")]
pub mod oled;

#[cfg(all(feature = "oled", not(feature = "breadboard")))]
pub mod sh1106;

#[cfg(feature = "leds")]
pub mod led;

#[cfg(feature = "board_v2")]
pub mod st7796;

#[cfg(feature = "board_v2")]
pub mod buttons_v2;

#[cfg(feature = "board_v2")]
pub mod usb_v2;
