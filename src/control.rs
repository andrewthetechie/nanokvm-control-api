use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tiny_http::{Response, StatusCode};

pub const VALID_IDS: [u8; 4] = [1, 2, 3, 4];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub current_input: Option<u8>,
    #[serde(default)]
    pub hard_power_state: HashMap<u8, u8>,
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
) -> Response<std::io::Cursor<Vec<u8>>> {
    match parse_id(id_str) {
        Ok(id) => {
            if let Err(e) = state_manager.update_current_input(id) {
                log::error!("Failed to update state: {}", e);
                return Response::from_string("Internal server error")
                    .with_status_code(StatusCode(500));
            }
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
) -> Response<std::io::Cursor<Vec<u8>>> {
    // Validate action (case-insensitive)
    let action_lower = action.to_lowercase();
    if action_lower != "on" && action_lower != "off" {
        return Response::from_string("Action must be 'on' or 'off'")
            .with_status_code(StatusCode(400));
    }

    match parse_id(id_str) {
        Ok(id) => {
            // Update state if it's a hard power action
            if kind == "hard" {
                let power_value = if action_lower == "on" { 1 } else { 0 };
                if let Err(e) = state_manager.update_hard_power_state(id, power_value) {
                    log::error!("Failed to update state: {}", e);
                    return Response::from_string("Internal server error")
                        .with_status_code(StatusCode(500));
                }
            }

            log::info!(
                "Power {} action {} triggered for {}",
                kind,
                action_lower,
                id
            );
            Response::from_string(format!(
                "Power {} action {} triggered for {}",
                kind, action_lower, id
            ))
        }
        Err(resp) => resp,
    }
}
