use anyhow::{Context, Result};
use tracing::debug;
use winreg::enums::*;
use winreg::RegKey;

/// Return the list of application name using the default microphone,
/// by reading the database register.
pub fn processes_owning_mic() -> Result<Vec<String>> {
    let mut res = Vec::new();
    let hklm = RegKey::predef(HKEY_CURRENT_USER);

    //Retrieve the "parent" key : under it, all application that can used the micro.
    let mic_info_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone\\NonPackaged";
    let cur_ver = hklm.open_subkey(mic_info_path).context(format!(
        "Parent key {:?} not found in base register",
        mic_info_path
    ))?;

    //Iterate on "child" keys
    for child_keys in cur_ver
        .enum_keys()
        .map(|x| x.unwrap_or_else(|_| panic!("No child keys found under {:?}", cur_ver)))
    {
        let process = cur_ver.open_subkey(child_keys.clone())?;

        //Iterate on key's "values". Keys name are the absolute path of the application with "/" replace by "#".
        // Example : C:#Program Files (x86)#ZoomRooms#bin#ZoomRooms.exe.
        for (name, value) in process
            .enum_values()
            .map(|x| x.unwrap_or_else(|_| panic!("No child keys found under {:?}", process)))
        {
            //Trigger on "LastUsedTimeStop" value : if equal to "0" (string), micro is currently in used by concerned application.
            if name == "LastUsedTimeStop" && value.to_string() == "0" {
                let process_path = child_keys.to_string();

                //Retrieve only application name (with extension)
                let process_path_splitted: Vec<&str> = process_path.split("#").collect();
                if let Some(process_name) = process_path_splitted.last() {
                    res.push(process_name.to_string());
                }
            }
        }
    }

    debug!("Process owning mic : {:?}", res);
    Ok(res)
}
