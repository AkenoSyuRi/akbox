use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
    pub product: Option<String>,
    pub device: Option<String>,
    pub transport_id: Option<String>,
}

impl AdbDevice {
    pub fn selector_label(&self, show_model: bool) -> String {
        if !show_model {
            return self.serial.clone();
        }
        let name = self.display_name();
        format!("{name:<24} {}", self.serial)
    }

    pub fn display_name(&self) -> String {
        for value in [
            self.model.as_deref(),
            self.product.as_deref(),
            self.device.as_deref(),
        ] {
            if let Some(name) = value.filter(|name| !name.is_empty()) {
                return name.to_string();
            }
        }
        "<unknown>".to_string()
    }
}

pub struct DeviceOption<'a> {
    pub device: &'a AdbDevice,
    pub show_model: bool,
}

impl fmt::Display for DeviceOption<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.device.selector_label(self.show_model))
    }
}

/// Parse `adb devices -l` stdout. Only lines with state `device` are returned.
pub fn parse_devices_output(output: &str) -> Vec<AdbDevice> {
    output
        .lines()
        .filter_map(parse_device_line)
        .filter(|d| d.state == "device")
        .collect()
}

fn parse_device_line(line: &str) -> Option<AdbDevice> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("List of devices") {
        return None;
    }

    let mut parts = line.split_whitespace();
    let serial = parts.next()?.to_string();
    let state = parts.next()?.to_string();

    let mut model = None;
    let mut product = None;
    let mut device = None;
    let mut transport_id = None;

    for token in parts {
        if let Some((key, value)) = token.split_once(':') {
            match key {
                "model" => model = Some(value.to_string()),
                "product" => product = Some(value.to_string()),
                "device" => device = Some(value.to_string()),
                "transport_id" => transport_id = Some(value.to_string()),
                _ => {}
            }
        }
    }

    Some(AdbDevice {
        serial,
        state,
        model,
        product,
        device,
        transport_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "List of devices attached\n\
123456F       device product:xxx model:OnePlus_Ace_3 device:phone transport_id:1\n\
emulator-5554 device product:sdk_gphone64 model:sdk_gphone64_x86_64 device:emu64x transport_id:2\n\
deadbeef      offline product:xxx model:Broken device:xxx transport_id:3\n";

    #[test]
    fn parses_online_devices_only() {
        let devices = parse_devices_output(SAMPLE);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].serial, "123456F");
        assert_eq!(devices[0].model.as_deref(), Some("OnePlus_Ace_3"));
        assert_eq!(devices[1].serial, "emulator-5554");
    }

    #[test]
    fn selector_label_uses_model_or_unknown() {
        let with_model = AdbDevice {
            serial: "123".into(),
            state: "device".into(),
            model: Some("Pixel".into()),
            product: None,
            device: None,
            transport_id: None,
        };
        assert!(with_model.selector_label(true).starts_with("Pixel"));
        assert_eq!(with_model.selector_label(false), "123");

        let without_model = AdbDevice {
            serial: "456".into(),
            state: "device".into(),
            model: None,
            product: None,
            device: None,
            transport_id: None,
        };
        assert!(without_model.selector_label(true).starts_with("<unknown>"));
    }

    #[test]
    fn display_name_falls_back_to_product() {
        let device = AdbDevice {
            serial: "789".into(),
            state: "device".into(),
            model: None,
            product: Some("my_product".into()),
            device: None,
            transport_id: None,
        };
        assert_eq!(device.display_name(), "my_product");
        assert!(device.selector_label(true).starts_with("my_product"));
    }
}
