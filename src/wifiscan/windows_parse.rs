pub(crate) fn extract_netsh_ssid(netsh_output: &str) -> Vec<String> {
    netsh_output
        .split('\n')
        .filter(|x| x.starts_with("SSID"))
        .map(|x| {
            x.split(':')
                .skip(1)
                .collect::<Vec<&str>>()
                .join(":")
                .trim()
                .to_owned()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    mod should {
        use super::*;
        use anyhow::Result;
        #[test]
        fn extract_expected_ssid() -> Result<()> {
            let res = r#"
Interface name : Wireless Network Connection
There are 22 networks currently visible.

SSID 1 : SKYXXXXX
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP

SSID 2 : SKYXXXXX
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP

SSID 3 : XXXXX
    Network type            : Infrastructure
    Authentication          : WPA2-Personal
    Encryption              : CCMP

SSID 4 : BTOpenzoneXXX
    Network type            : Infrastructure
    Authentication          : Open
    Encryption              : None
"#;

            assert_eq!(
                extract_netsh_ssid(res),
                ["SKYXXXXX", "SKYXXXXX", "XXXXX", "BTOpenzoneXXX"]
            );
            Ok(())
        }
    }
}
