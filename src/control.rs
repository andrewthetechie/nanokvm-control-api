use tiny_http::{Response, StatusCode};

pub const VALID_IDS: [u8; 4] = [1, 2, 3, 4];

pub fn parse_id(id_str: &str) -> Result<u8, Response<std::io::Cursor<Vec<u8>>>> {
    if let Ok(id) = id_str.parse::<u8>() {
        if VALID_IDS.contains(&id) {
            return Ok(id);
        }
    }

    Err(Response::from_string("ID must be integer 1-4")
        .with_status_code(StatusCode(400)))
}

pub fn handle_input(id_str: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    match parse_id(id_str) {
        Ok(id) => {
            println!("Setting input to {}", id);
            Response::from_string(format!("Input {} selected", id))
        }
        Err(resp) => resp,
    }
}

pub fn handle_power(kind: &str, id_str: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    match parse_id(id_str) {
        Ok(id) => {
            println!("Power {} action triggered for {}", kind, id);
            Response::from_string(format!("Power {} action triggered for {}", kind, id))
        }
        Err(resp) => resp,
    }
}
