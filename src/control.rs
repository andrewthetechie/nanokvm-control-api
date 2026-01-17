use crate::config::InputPinConfig;
use linux_embedded_hal::I2cdev;
use pcf857x::{OutputPin, Pcf8574, SlaveAddr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tiny_http::{Response, StatusCode};

pub const VALID_IDS: [u8; 4] = [1, 2, 3, 4];

/// Get the current power state for all instances.
/// Currently stubbed out to return 0 (off) for all instances.
pub fn get_power_state() -> HashMap<u8, u8> {
    let mut power_state = HashMap::new();
    for id in VALID_IDS {
        power_state.insert(id, 0);
    }
    power_state
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub current_input: Option<u8>,
    #[serde(default)]
    pub hard_power_state: HashMap<u8, u8>,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_input: Option<u8>,
    pub hard_power: HashMap<String, String>,
}

impl From<State> for StatusResponse {
    fn from(state: State) -> Self {
        let mut hard_power = HashMap::new();
        for id in VALID_IDS {
            let value = state.hard_power_state.get(&id).copied().unwrap_or(0);
            let status = if value == 1 { "on" } else { "off" };
            hard_power.insert(id.to_string(), status.to_string());
        }
        StatusResponse {
            current_input: state.current_input,
            hard_power,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        let mut hard_power_state = HashMap::new();
        for id in VALID_IDS {
            hard_power_state.insert(id, 0);
        }
        State {
            current_input: None,
            hard_power_state,
        }
    }
}

pub struct StateManager {
    state: Mutex<State>,
    storage_path: String,
}

impl StateManager {
    pub fn new(storage_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let state = Self::load_state(&storage_path)?;
        Ok(StateManager {
            state: Mutex::new(state),
            storage_path,
        })
    }

    fn load_state(storage_path: &str) -> Result<State, Box<dyn std::error::Error>> {
        let path = Path::new(storage_path);
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let state: State = serde_json::from_str(&contents)?;
            Ok(state)
        } else {
            let state = State::default();
            // Save the default state to create the file
            let contents = serde_json::to_string_pretty(&state)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, contents)?;
            Ok(state)
        }
    }

    fn save_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.state.lock().unwrap();
        let contents = serde_json::to_string_pretty(&*state)?;
        let path = Path::new(&self.storage_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn update_current_input(&self, id: u8) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.state.lock().unwrap();
        state.current_input = Some(id);
        drop(state); // Release lock before file I/O
        self.save_state()?;
        Ok(())
    }

    pub fn update_hard_power_state(
        &self,
        id: u8,
        value: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.state.lock().unwrap();
        state.hard_power_state.insert(id, value);
        drop(state); // Release lock before file I/O
        self.save_state()?;
        Ok(())
    }

    pub fn get_state(&self) -> State {
        let state = self.state.lock().unwrap();
        state.clone()
    }

    pub fn clear_and_regenerate_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        let power_state = get_power_state();
        let mut state = self.state.lock().unwrap();
        state.current_input = None;
        state.hard_power_state = power_state;
        drop(state); // Release lock before file I/O
        self.save_state()?;
        Ok(())
    }
}

/// Initialize the PCF8574 device using the I2C bus and address from config.
/// Returns a thread-safe shared reference to the device.
pub fn init_pcf8574(
    i2c_bus: &str,
    i2c_address_str: &str,
) -> Result<Arc<Mutex<Pcf8574<I2cdev>>>, Box<dyn std::error::Error>> {
    // Parse the I2C address from hex string (e.g., "0x20" -> 0x20)
    let address = if i2c_address_str.starts_with("0x") || i2c_address_str.starts_with("0X") {
        u8::from_str_radix(&i2c_address_str[2..], 16)?
    } else {
        i2c_address_str.parse::<u8>()?
    };

    log::info!(
        "Initializing PCF8574 on bus {} with address 0x{:02x}",
        i2c_bus,
        address
    );

    // Open the I2C device
    let i2c = I2cdev::new(i2c_bus)?;

    // For PCF8574, the base address is 0x20, and A0/A1/A2 pins determine the final address
    // Extract the address bits: address = 0x20 + (A2 << 2) + (A1 << 1) + A0
    // So: address_bits = address - 0x20, then extract individual bits
    let address_offset = address.wrapping_sub(0x20);
    let a0 = (address_offset & 0x01) != 0;
    let a1 = (address_offset & 0x02) != 0;
    let a2 = (address_offset & 0x04) != 0;

    log::debug!("PCF8574 address bits: A0={}, A1={}, A2={}", a0, a1, a2);

    // Create SlaveAddr from the address bits
    let slave_addr = SlaveAddr::Alternative(a0, a1, a2);
    let pcf8574 = Pcf8574::new(i2c, slave_addr);

    log::info!("PCF8574 device created successfully");
    Ok(Arc::new(Mutex::new(pcf8574)))
}

pub fn parse_id(id_str: &str) -> Result<u8, Response<std::io::Cursor<Vec<u8>>>> {
    if let Ok(id) = id_str.parse::<u8>()
        && VALID_IDS.contains(&id)
    {
        return Ok(id);
    }

    Err(Response::from_string("ID must be integer 1-4").with_status_code(StatusCode(400)))
}

pub fn handle_input(
    state_manager: &StateManager,
    id_str: &str,
    pcf8574: Arc<Mutex<Pcf8574<I2cdev>>>,
    input_config: &HashMap<u8, InputPinConfig>,
    button_press_delay_ms: f32,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match parse_id(id_str) {
        Ok(id) => {
            // Look up the input configuration
            let pin_config = match input_config.get(&id) {
                Some(config) => config,
                None => {
                    log::error!("Input ID {} not found in USB input config", id);
                    return Response::from_string(format!("Input ID {} not configured", id))
                        .with_status_code(StatusCode(400));
                }
            };

            // Update state first
            if let Err(e) = state_manager.update_current_input(id) {
                log::error!("Failed to update state: {}", e);
                return Response::from_string("Internal server error")
                    .with_status_code(StatusCode(500));
            }

            // Toggle the pin using PCF8574
            // We need to release the lock before sleeping, so we'll do this in two steps
            // First: set to pushed_state
            {
                let device = match pcf8574.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        log::error!("Failed to lock PCF8574 device: {}", e);
                        return Response::from_string("Internal server error")
                            .with_status_code(StatusCode(500));
                    }
                };

                // Split the device to access individual pins
                let mut pins = device.split();

                // Set pin to pushed_state - we need to match on pin number and call set_high/set_low directly
                // since each pin type is different, we can't store them in a single variable
                if pin_config.pin > 7 {
                    log::error!("Invalid pin number: {}", pin_config.pin);
                    return Response::from_string(format!(
                        "Invalid pin number: {}",
                        pin_config.pin
                    ))
                    .with_status_code(StatusCode(500));
                }

                let set_result = match pin_config.pin {
                    0 => {
                        if pin_config.pushed_state == 1 {
                            pins.p0.set_high()
                        } else {
                            pins.p0.set_low()
                        }
                    }
                    1 => {
                        if pin_config.pushed_state == 1 {
                            pins.p1.set_high()
                        } else {
                            pins.p1.set_low()
                        }
                    }
                    2 => {
                        if pin_config.pushed_state == 1 {
                            pins.p2.set_high()
                        } else {
                            pins.p2.set_low()
                        }
                    }
                    3 => {
                        if pin_config.pushed_state == 1 {
                            pins.p3.set_high()
                        } else {
                            pins.p3.set_low()
                        }
                    }
                    4 => {
                        if pin_config.pushed_state == 1 {
                            pins.p4.set_high()
                        } else {
                            pins.p4.set_low()
                        }
                    }
                    5 => {
                        if pin_config.pushed_state == 1 {
                            pins.p5.set_high()
                        } else {
                            pins.p5.set_low()
                        }
                    }
                    6 => {
                        if pin_config.pushed_state == 1 {
                            pins.p6.set_high()
                        } else {
                            pins.p6.set_low()
                        }
                    }
                    7 => {
                        if pin_config.pushed_state == 1 {
                            pins.p7.set_high()
                        } else {
                            pins.p7.set_low()
                        }
                    }
                    _ => unreachable!(), // Already checked above
                };

                if let Err(e) = set_result {
                    log::error!(
                        "Failed to set pin {} to pushed state: {:?}",
                        pin_config.pin,
                        e
                    );
                    return Response::from_string("Internal server error")
                        .with_status_code(StatusCode(500));
                }
            } // Lock is released here

            // Wait for the button press delay (lock is released during sleep)
            thread::sleep(Duration::from_millis(button_press_delay_ms as u64));

            // Second: set to inverse of pushed_state
            {
                let device = match pcf8574.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        log::error!("Failed to lock PCF8574 device: {}", e);
                        return Response::from_string("Internal server error")
                            .with_status_code(StatusCode(500));
                    }
                };

                let mut pins = device.split();

                // Set to inverse state
                let set_result = match pin_config.pin {
                    0 => {
                        if pin_config.pushed_state == 1 {
                            pins.p0.set_low()
                        } else {
                            pins.p0.set_high()
                        }
                    }
                    1 => {
                        if pin_config.pushed_state == 1 {
                            pins.p1.set_low()
                        } else {
                            pins.p1.set_high()
                        }
                    }
                    2 => {
                        if pin_config.pushed_state == 1 {
                            pins.p2.set_low()
                        } else {
                            pins.p2.set_high()
                        }
                    }
                    3 => {
                        if pin_config.pushed_state == 1 {
                            pins.p3.set_low()
                        } else {
                            pins.p3.set_high()
                        }
                    }
                    4 => {
                        if pin_config.pushed_state == 1 {
                            pins.p4.set_low()
                        } else {
                            pins.p4.set_high()
                        }
                    }
                    5 => {
                        if pin_config.pushed_state == 1 {
                            pins.p5.set_low()
                        } else {
                            pins.p5.set_high()
                        }
                    }
                    6 => {
                        if pin_config.pushed_state == 1 {
                            pins.p6.set_low()
                        } else {
                            pins.p6.set_high()
                        }
                    }
                    7 => {
                        if pin_config.pushed_state == 1 {
                            pins.p7.set_low()
                        } else {
                            pins.p7.set_high()
                        }
                    }
                    _ => unreachable!(), // We already validated this above
                };

                if let Err(e) = set_result {
                    log::error!(
                        "Failed to set pin {} to inverse state: {:?}",
                        pin_config.pin,
                        e
                    );
                    return Response::from_string("Internal server error")
                        .with_status_code(StatusCode(500));
                }
            } // Lock is released here

            log::info!("Setting input to {}", id);
            Response::from_string(format!("Input {} selected", id))
        }
        Err(resp) => resp,
    }
}

pub fn handle_power(
    state_manager: &StateManager,
    kind: &str,
    id_str: &str,
    action: &str,
    pcf8574: Arc<Mutex<Pcf8574<I2cdev>>>,
    power_soft_config: &HashMap<u8, InputPinConfig>,
    power_hard_config: &HashMap<u8, crate::config::HardPowerPinConfig>,
    hard_power_delay_ms: f32,
) -> Response<std::io::Cursor<Vec<u8>>> {
    match kind {
        "soft" => handle_power_soft(state_manager, id_str, action),
        "hard" => handle_power_hard(
            state_manager,
            id_str,
            action,
            pcf8574,
            power_hard_config,
            hard_power_delay_ms,
        ),
        _ => Response::from_string(format!("Invalid power kind: {}", kind))
            .with_status_code(StatusCode(400)),
    }
}

fn handle_power_soft(
    state_manager: &StateManager,
    id_str: &str,
    action: &str,
) -> Response<std::io::Cursor<Vec<u8>>> {
    // Stub implementation - will be built later
    match parse_id(id_str) {
        Ok(id) => {
            log::info!("Soft power action {} for {} (stubbed)", action, id);
            Response::from_string(format!(
                "Soft power action {} triggered for {} (stubbed)",
                action, id
            ))
        }
        Err(resp) => resp,
    }
}

fn handle_power_hard(
    state_manager: &StateManager,
    id_str: &str,
    action: &str,
    pcf8574: Arc<Mutex<Pcf8574<I2cdev>>>,
    power_hard_config: &HashMap<u8, crate::config::HardPowerPinConfig>,
    hard_power_delay_ms: f32,
) -> Response<std::io::Cursor<Vec<u8>>> {
    // Validate action (case-insensitive)
    let action_lower = action.to_lowercase();
    if action_lower != "on" && action_lower != "off" && action_lower != "toggle" {
        return Response::from_string("Action must be 'on', 'off', or 'toggle'")
            .with_status_code(StatusCode(400));
    }

    match parse_id(id_str) {
        Ok(id) => {
            // Look up the pin configuration
            let pin_config = match power_hard_config.get(&id) {
                Some(config) => config,
                None => {
                    log::error!("Power ID {} not found in power hard config", id);
                    return Response::from_string(format!("Power ID {} not configured", id))
                        .with_status_code(StatusCode(400));
                }
            };

            // Validate pin number
            if pin_config.pin > 7 {
                log::error!("Invalid pin number: {}", pin_config.pin);
                return Response::from_string(format!("Invalid pin number: {}", pin_config.pin))
                    .with_status_code(StatusCode(500));
            }

            // Handle different actions
            match action_lower.as_str() {
                "off" => {
                    // Set pin to opposite of on_state
                    let target_state = if pin_config.on_state == 1 {
                        false
                    } else {
                        true
                    };

                    let device = match pcf8574.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            log::error!("Failed to lock PCF8574 device: {}", e);
                            return Response::from_string("Internal server error")
                                .with_status_code(StatusCode(500));
                        }
                    };

                    let mut pins = device.split();
                    let set_result = match pin_config.pin {
                        0 => {
                            if target_state {
                                pins.p0.set_high()
                            } else {
                                pins.p0.set_low()
                            }
                        }
                        1 => {
                            if target_state {
                                pins.p1.set_high()
                            } else {
                                pins.p1.set_low()
                            }
                        }
                        2 => {
                            if target_state {
                                pins.p2.set_high()
                            } else {
                                pins.p2.set_low()
                            }
                        }
                        3 => {
                            if target_state {
                                pins.p3.set_high()
                            } else {
                                pins.p3.set_low()
                            }
                        }
                        4 => {
                            if target_state {
                                pins.p4.set_high()
                            } else {
                                pins.p4.set_low()
                            }
                        }
                        5 => {
                            if target_state {
                                pins.p5.set_high()
                            } else {
                                pins.p5.set_low()
                            }
                        }
                        6 => {
                            if target_state {
                                pins.p6.set_high()
                            } else {
                                pins.p6.set_low()
                            }
                        }
                        7 => {
                            if target_state {
                                pins.p7.set_high()
                            } else {
                                pins.p7.set_low()
                            }
                        }
                        _ => unreachable!(), // Already validated above
                    };

                    if let Err(e) = set_result {
                        log::error!("Failed to set pin {} to off state: {:?}", pin_config.pin, e);
                        return Response::from_string("Internal server error")
                            .with_status_code(StatusCode(500));
                    }

                    // Update state
                    if let Err(e) = state_manager.update_hard_power_state(id, 0) {
                        log::error!("Failed to update state: {}", e);
                        return Response::from_string("Internal server error")
                            .with_status_code(StatusCode(500));
                    }

                    log::info!("Power hard off triggered for {}", id);
                    Response::from_string(format!("Power hard off triggered for {}", id))
                }

                "on" => {
                    // Set pin to on_state value
                    let target_state = pin_config.on_state == 1;

                    let device = match pcf8574.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            log::error!("Failed to lock PCF8574 device: {}", e);
                            return Response::from_string("Internal server error")
                                .with_status_code(StatusCode(500));
                        }
                    };

                    let mut pins = device.split();
                    let set_result = match pin_config.pin {
                        0 => {
                            if target_state {
                                pins.p0.set_high()
                            } else {
                                pins.p0.set_low()
                            }
                        }
                        1 => {
                            if target_state {
                                pins.p1.set_high()
                            } else {
                                pins.p1.set_low()
                            }
                        }
                        2 => {
                            if target_state {
                                pins.p2.set_high()
                            } else {
                                pins.p2.set_low()
                            }
                        }
                        3 => {
                            if target_state {
                                pins.p3.set_high()
                            } else {
                                pins.p3.set_low()
                            }
                        }
                        4 => {
                            if target_state {
                                pins.p4.set_high()
                            } else {
                                pins.p4.set_low()
                            }
                        }
                        5 => {
                            if target_state {
                                pins.p5.set_high()
                            } else {
                                pins.p5.set_low()
                            }
                        }
                        6 => {
                            if target_state {
                                pins.p6.set_high()
                            } else {
                                pins.p6.set_low()
                            }
                        }
                        7 => {
                            if target_state {
                                pins.p7.set_high()
                            } else {
                                pins.p7.set_low()
                            }
                        }
                        _ => unreachable!(), // Already validated above
                    };

                    if let Err(e) = set_result {
                        log::error!("Failed to set pin {} to on state: {:?}", pin_config.pin, e);
                        return Response::from_string("Internal server error")
                            .with_status_code(StatusCode(500));
                    }

                    // Update state
                    if let Err(e) = state_manager.update_hard_power_state(id, 1) {
                        log::error!("Failed to update state: {}", e);
                        return Response::from_string("Internal server error")
                            .with_status_code(StatusCode(500));
                    }

                    log::info!("Power hard on triggered for {}", id);
                    Response::from_string(format!("Power hard on triggered for {}", id))
                }

                "toggle" => {
                    // First: set to off state (opposite of on_state)
                    let off_state = if pin_config.on_state == 1 {
                        false
                    } else {
                        true
                    };

                    {
                        let device = match pcf8574.lock() {
                            Ok(guard) => guard,
                            Err(e) => {
                                log::error!("Failed to lock PCF8574 device: {}", e);
                                return Response::from_string("Internal server error")
                                    .with_status_code(StatusCode(500));
                            }
                        };

                        let mut pins = device.split();
                        let set_result = match pin_config.pin {
                            0 => {
                                if off_state {
                                    pins.p0.set_high()
                                } else {
                                    pins.p0.set_low()
                                }
                            }
                            1 => {
                                if off_state {
                                    pins.p1.set_high()
                                } else {
                                    pins.p1.set_low()
                                }
                            }
                            2 => {
                                if off_state {
                                    pins.p2.set_high()
                                } else {
                                    pins.p2.set_low()
                                }
                            }
                            3 => {
                                if off_state {
                                    pins.p3.set_high()
                                } else {
                                    pins.p3.set_low()
                                }
                            }
                            4 => {
                                if off_state {
                                    pins.p4.set_high()
                                } else {
                                    pins.p4.set_low()
                                }
                            }
                            5 => {
                                if off_state {
                                    pins.p5.set_high()
                                } else {
                                    pins.p5.set_low()
                                }
                            }
                            6 => {
                                if off_state {
                                    pins.p6.set_high()
                                } else {
                                    pins.p6.set_low()
                                }
                            }
                            7 => {
                                if off_state {
                                    pins.p7.set_high()
                                } else {
                                    pins.p7.set_low()
                                }
                            }
                            _ => unreachable!(), // Already validated above
                        };

                        if let Err(e) = set_result {
                            log::error!(
                                "Failed to set pin {} to off state: {:?}",
                                pin_config.pin,
                                e
                            );
                            return Response::from_string("Internal server error")
                                .with_status_code(StatusCode(500));
                        }
                    } // Lock is released here

                    // Wait for the hard power delay
                    thread::sleep(Duration::from_millis(hard_power_delay_ms as u64));

                    // Second: set to on state
                    {
                        let device = match pcf8574.lock() {
                            Ok(guard) => guard,
                            Err(e) => {
                                log::error!("Failed to lock PCF8574 device: {}", e);
                                return Response::from_string("Internal server error")
                                    .with_status_code(StatusCode(500));
                            }
                        };

                        let mut pins = device.split();
                        let target_state = pin_config.on_state == 1;
                        let set_result = match pin_config.pin {
                            0 => {
                                if target_state {
                                    pins.p0.set_high()
                                } else {
                                    pins.p0.set_low()
                                }
                            }
                            1 => {
                                if target_state {
                                    pins.p1.set_high()
                                } else {
                                    pins.p1.set_low()
                                }
                            }
                            2 => {
                                if target_state {
                                    pins.p2.set_high()
                                } else {
                                    pins.p2.set_low()
                                }
                            }
                            3 => {
                                if target_state {
                                    pins.p3.set_high()
                                } else {
                                    pins.p3.set_low()
                                }
                            }
                            4 => {
                                if target_state {
                                    pins.p4.set_high()
                                } else {
                                    pins.p4.set_low()
                                }
                            }
                            5 => {
                                if target_state {
                                    pins.p5.set_high()
                                } else {
                                    pins.p5.set_low()
                                }
                            }
                            6 => {
                                if target_state {
                                    pins.p6.set_high()
                                } else {
                                    pins.p6.set_low()
                                }
                            }
                            7 => {
                                if target_state {
                                    pins.p7.set_high()
                                } else {
                                    pins.p7.set_low()
                                }
                            }
                            _ => unreachable!(), // Already validated above
                        };

                        if let Err(e) = set_result {
                            log::error!(
                                "Failed to set pin {} to on state: {:?}",
                                pin_config.pin,
                                e
                            );
                            return Response::from_string("Internal server error")
                                .with_status_code(StatusCode(500));
                        }
                    } // Lock is released here

                    // Update state
                    if let Err(e) = state_manager.update_hard_power_state(id, 1) {
                        log::error!("Failed to update state: {}", e);
                        return Response::from_string("Internal server error")
                            .with_status_code(StatusCode(500));
                    }

                    log::info!("Power hard toggle triggered for {}", id);
                    Response::from_string(format!("Power hard toggle triggered for {}", id))
                }

                _ => unreachable!(), // Already validated above
            }
        }
        Err(resp) => resp,
    }
}
