use serde::{Deserialize, Serialize};
use std::ffi::{c_char, CStr, CString};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    pub parameters: Vec<PluginParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub param_type: String,
}

#[no_mangle]
pub extern "C" fn plugin_get_info() -> *mut c_char {
    let info = PluginInfo {
        name: "Example Plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "An example plugin for SerialTap".to_string(),
        author: "SerialTap Team".to_string(),
    };

    let json = serde_json::to_string(&info).unwrap();
    CString::new(json).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn plugin_get_commands() -> *mut c_char {
    let commands = vec![
        PluginCommand {
            name: "echo".to_string(),
            description: "Echo back the input data".to_string(),
            parameters: vec![PluginParameter {
                name: "data".to_string(),
                description: "Data to echo".to_string(),
                required: true,
                param_type: "string".to_string(),
            }],
        },
        PluginCommand {
            name: "timestamp".to_string(),
            description: "Get current timestamp".to_string(),
            parameters: vec![],
        },
    ];

    let json = serde_json::to_string(&commands).unwrap();
    CString::new(json).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn plugin_execute(command: *const c_char, params: *const c_char) -> *mut c_char {
    let command = unsafe {
        if command.is_null() {
            return CString::new("{\"error\": \"Null command\"}").unwrap().into_raw();
        }
        CStr::from_ptr(command).to_string_lossy().to_string()
    };

    let params = unsafe {
        if params.is_null() {
            "{}".to_string()
        } else {
            CStr::from_ptr(params).to_string_lossy().to_string()
        }
    };

    let result = match command.as_str() {
        "echo" => {
            let parsed: serde_json::Value = serde_json::from_str(&params).unwrap_or_default();
            let data = parsed["data"].as_str().unwrap_or("No data");
            serde_json::json!({
                "success": true,
                "result": data
            })
        }
        "timestamp" => {
            let now = chrono::Utc::now();
            serde_json::json!({
                "success": true,
                "timestamp": now.to_rfc3339()
            })
        }
        _ => {
            serde_json::json!({
                "success": false,
                "error": format!("Unknown command: {}", command)
            })
        }
    };

    CString::new(result.to_string()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn plugin_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_info() {
        let ptr = plugin_get_info();
        assert!(!ptr.is_null());

        let info_str = unsafe { CStr::from_ptr(ptr).to_string_lossy() };
        let info: PluginInfo = serde_json::from_str(&info_str).unwrap();

        assert_eq!(info.name, "Example Plugin");
        assert_eq!(info.version, "0.1.0");

        plugin_free_string(ptr);
    }

    #[test]
    fn test_plugin_commands() {
        let ptr = plugin_get_commands();
        assert!(!ptr.is_null());

        let commands_str = unsafe { CStr::from_ptr(ptr).to_string_lossy() };
        let commands: Vec<PluginCommand> = serde_json::from_str(&commands_str).unwrap();

        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].name, "echo");

        plugin_free_string(ptr);
    }

    #[test]
    fn test_plugin_execute_echo() {
        let command = CString::new("echo").unwrap();
        let params = CString::new(r#"{"data": "hello"}"#).unwrap();

        let ptr = plugin_execute(command.as_ptr(), params.as_ptr());
        assert!(!ptr.is_null());

        let result_str = unsafe { CStr::from_ptr(ptr).to_string_lossy() };
        let result: serde_json::Value = serde_json::from_str(&result_str).unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["result"], "hello");

        plugin_free_string(ptr);
    }

    #[test]
    fn test_plugin_execute_unknown() {
        let command = CString::new("unknown").unwrap();
        let params = CString::new("{}").unwrap();

        let ptr = plugin_execute(command.as_ptr(), params.as_ptr());
        assert!(!ptr.is_null());

        let result_str = unsafe { CStr::from_ptr(ptr).to_string_lossy() };
        let result: serde_json::Value = serde_json::from_str(&result_str).unwrap();

        assert_eq!(result["success"], false);

        plugin_free_string(ptr);
    }
}
