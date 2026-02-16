use anyhow::{Context, Result};
use std::ffi::OsStr;
use tracing::{debug, error};
use winreg::enums::*;
use winreg::RegKey;

/// Return the list of application name using the default microphone,
/// by reading the database register.
pub fn processes_owning_mic() -> Result<Vec<String>> {
    let hklm = RegKey::predef(HKEY_CURRENT_USER);
    processes_owning_mic_in_registry(hklm)
}

fn processes_owning_mic_in_subkey<T: AsRef<OsStr> + Clone + ToString + std::fmt::Debug>(
    key: RegKey,
    path: T,
) -> Result<Vec<String>> {
    let mut res = Vec::new();
    if let Ok(subkey) = key.open_subkey(path.clone()) {
        let processes = subkey.enum_values();

        //Iterate on path's "values". Keys name are the absolute path of the application with "/" replace by "#".
        // Example : C:#Program Files (x86)#ZoomRooms#bin#ZoomRooms.exe.
        for process in processes {
            if let Ok((name, value)) = process {
                //Trigger on "LastUsedTimeStop" value : if equal to "0" (string), micro is currently in used by concerned application.
                if name == "LastUsedTimeStop" && value.to_string() == "0" {
                    let process_path = path.to_string();

                    //Retrieve only application name (with extension)
                    let process_path_splitted: Vec<&str> = process_path.split("#").collect();
                    if let Some(process_name) = process_path_splitted.last() {
                        res.push(process_name.to_string());
                    }
                }
            } else {
                error!("Unable to open process: {:?}", process);
            }
        }
        let keys = subkey.enum_keys();
        for child_key in keys {
            if let Ok(path) = child_key {
                debug!("Recusively analyses {path}");
                res.extend(processes_owning_mic_in_subkey(
                    subkey.open_subkey("")?,
                    path,
                )?);
            } else {
                error!("Unable to open subkey: {:?} ", child_key);
            }
        }
    } else {
        error!("Unable to open subkey: {:?}", path);
    }
    Ok(res)
}

fn processes_owning_mic_in_registry(hklm: RegKey) -> Result<Vec<String>> {
    //Retrieve the "parent" key : under it, all application that can used the micro.
    let mic_info_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone";
    let mic_used_key = hklm
        .open_subkey(mic_info_path)
        .context(format!("Opening key {:?} in base register", mic_info_path))?;

    let res = processes_owning_mic_in_subkey(mic_used_key, "")?;

    debug!("Process owning mic : {:?}", res);
    Ok(res)
}

#[cfg(test)]
mod processes_owning_mic_should {
    use super::*;
    use anyhow::Result;
    use test_log::test; // Automatically trace tests

    #[test]
    fn return_empty_vec_if_none_is_active() -> Result<()> {
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let base_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion";
        // \\CapabilityAccessManager\\ConsentStore\\microphone\\NonPackaged";
        let key = hklm
            .open_subkey(base_path)
            .with_context(|| format!("open subkey {base_path}"))?;
        let (key, _) = key
            .create_subkey("CapabilityAccessManager")
            .with_context(|| format!("create subkey {base_path}/CapabilityAccessManager"))?;
        let (key, _) = key.create_subkey("ConsentStore").with_context(|| {
            format!("create subkey {base_path}/CapabilityAccessManager/ConsentStore")
        })?;
        key.delete_subkey_all("microphone")
            .with_context(|| {
                format!(
                    "delete subkey all {base_path}/CapabilityAccessManager/ConsentStore/microphone"
                )
            })
            .ok(); // Start from empty hierarchy
        let (mic, _) = key.create_subkey("microphone").with_context(|| {
            format!("delete subkey all {base_path}/CapabilityAccessManager/ConsentStore/microphone")
        })?;
        let (teams, _) = mic.create_subkey("APP#Truc#Teams.exe")
            .with_context(|| format!("delete subkey all {base_path}/CapabilityAccessManager/ConsentStore/microphone/APP#Truc#Teams.exe"))?;
        teams.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = mic.create_subkey("APP#Truc#Other.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (np, _) = mic.create_subkey("NonPackaged")?;
        let (key, _) = np.create_subkey("APP#Truc#Firefox.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = np.create_subkey("APP#Truc#Zoom.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let res = processes_owning_mic_in_registry(hklm)?;
        assert_eq!(res.len(), 0);
        Ok(())
    }
    #[test]
    fn return_non_packaged_app_if_active() -> Result<()> {
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let base_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion";
        // \\CapabilityAccessManager\\ConsentStore\\microphone\\NonPackaged";
        let key = hklm
            .open_subkey(base_path)
            .with_context(|| format!("open subkey {base_path}"))?;
        let (key, _) = key.create_subkey("CapabilityAccessManager")?;
        let (key, _) = key.create_subkey("ConsentStore")?;
        key.delete_subkey_all("microphone").ok(); // Start from empty hierarchy
        let (mic, _) = key.create_subkey("microphone")?;
        let (teams, _) = mic.create_subkey("APP#Truc#Teams.exe")?;
        teams.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = mic.create_subkey("APP#Truc#Other.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (np, _) = mic.create_subkey("NonPackaged")?;
        let (key, _) = np.create_subkey("APP#Truc#Firefox.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = np.create_subkey("APP#Truc#Zoom.exe")?;
        key.set_value("LastUsedTimeStop", &"0")?;
        let res = processes_owning_mic_in_registry(hklm)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], "Zoom.exe".to_string());
        Ok(())
    }

    #[test]
    fn return_other_category_app_if_active() -> Result<()> {
        // If a subkey other than NonPackaged contains app using microphpne
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let base_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion";
        // \\CapabilityAccessManager\\ConsentStore\\microphone\\NonPackaged";
        let key = hklm
            .open_subkey(base_path)
            .with_context(|| format!("open subkey {base_path}"))?;
        let (key, _) = key.create_subkey("CapabilityAccessManager")?;
        let (key, _) = key.create_subkey("ConsentStore")?;
        key.delete_subkey_all("microphone").ok(); // Start from empty hierarchy
        let (mic, _) = key.create_subkey("microphone")?;
        let (teams, _) = mic.create_subkey("APP#Truc#Teams.exe")?;
        teams.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = mic.create_subkey("APP#Truc#Other.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (np, _) = mic.create_subkey("NonPackaged")?;
        let (key, _) = np.create_subkey("APP#Truc#Firefox.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = np.create_subkey("APP#Truc#Zoom.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (other, _) = mic.create_subkey("Other")?;
        let (key, _) = other.create_subkey("APP#Truc#Blob.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = other.create_subkey("APP#Truc#MicApp.exe")?;
        key.set_value("LastUsedTimeStop", &"0")?;
        let res = processes_owning_mic_in_registry(hklm)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], "MicApp.exe".to_string());
        Ok(())
    }

    #[test]
    fn return_packaged_app_if_active() -> Result<()> {
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let base_path = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion";
        // \\CapabilityAccessManager\\ConsentStore\\microphone\\NonPackaged";
        let key = hklm
            .open_subkey(base_path)
            .with_context(|| format!("open subkey {base_path}"))?;
        let (key, _) = key.create_subkey("CapabilityAccessManager")?;
        let (key, _) = key.create_subkey("ConsentStore")?;
        key.delete_subkey_all("microphone").ok(); // Start from empty hierarchy
        let (mic, _) = key.create_subkey("microphone")?;
        let (teams, _) = mic.create_subkey("APP#Truc#Teams.exe")?;
        teams.set_value("LastUsedTimeStop", &"0")?;
        let (key, _) = mic.create_subkey("APP#Truc#Other.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (np, _) = mic.create_subkey("NonPackaged")?;
        let (key, _) = np.create_subkey("APP#Truc#Firefox.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let (key, _) = np.create_subkey("APP#Truc#Zoom.exe")?;
        key.set_value("LastUsedTimeStop", &"1")?;
        let res = processes_owning_mic_in_registry(hklm)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], "Teams.exe".to_string());
        Ok(())
    }
}
