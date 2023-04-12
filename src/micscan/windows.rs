use anyhow::{Context, Result};
use tracing::{debug, error};
use winreg::enums::*;
use winreg::RegKey;

/// Return the list of application name using the default microphone,
/// by reading the database register.
pub fn processes_owning_mic() -> Result<Vec<String>> {
    let mut res = Vec::new();
    let hklm = RegKey::predef(HKEY_CURRENT_USER);

    //Retrieve the "parent" key : under it, all application that can used the micro.
    let mic_info_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone\\NonPackaged";
    let mic_used_key = hklm
        .open_subkey(mic_info_path)
        .context(format!("Opening key {:?} in base register", mic_info_path))?;

    //Iterate on "child" keys
    let keys = mic_used_key.enum_keys();
    for child_key in keys {
        if let Ok(key) = child_key {
            if let Ok(subkey) = mic_used_key.open_subkey(key.clone()) {
                let processes = subkey.enum_values();

                //Iterate on key's "values". Keys name are the absolute path of the application with "/" replace by "#".
                // Example : C:#Program Files (x86)#ZoomRooms#bin#ZoomRooms.exe.
                for process in processes {
                    if let Ok((name, value)) = process {
                        //Trigger on "LastUsedTimeStop" value : if equal to "0" (string), micro is currently in used by concerned application.
                        if name == "LastUsedTimeStop" && value.to_string() == "0" {
                            let process_path = key.to_string();

                            //Retrieve only application name (with extension)
                            let process_path_splitted: Vec<&str> =
                                process_path.split("#").collect();
                            if let Some(process_name) = process_path_splitted.last() {
                                res.push(process_name.to_string());
                            }
                        }
                    } else {
                        error!("Unable to open process: {:?}", process);
                    }
                }
            } else {
                error!("Unable to open subkey: {:?}", key);
            }
        } else {
            error!("Unable to open subkey: {:?} ", child_key);
        }
    }

    debug!("Process owning mic : {:?}", res);
    Ok(res)
}
