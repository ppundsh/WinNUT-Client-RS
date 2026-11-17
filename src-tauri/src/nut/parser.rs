use crate::nut::models::UpsData;

/// Splits one NUT protocol line into words. Double-quoted words may contain spaces and the
/// escapes `\"` and `\\` (NUT network protocol, "quoting" section).
pub fn tokenize(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.trim_end_matches(['\r', '\n']).chars().peekable();
    loop {
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        let Some(&first) = chars.peek() else { break };
        let mut word = String::new();
        if first == '"' {
            chars.next();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(n) = chars.next() {
                            word.push(n);
                        }
                    }
                    '"' => break,
                    _ => word.push(c),
                }
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c == ' ' {
                    break;
                }
                word.push(c);
                chars.next();
            }
        }
        out.push(word);
    }
    out
}

/// Returns the server error code if the response is an `ERR <code>` line.
pub fn error_code(response: &str) -> Option<String> {
    let first = response.lines().next()?.trim();
    let rest = first.strip_prefix("ERR")?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    Some(rest.trim().split(' ').next().unwrap_or("UNKNOWN").to_string())
}

/// Parses a `LIST VAR <ups>` response. Only `VAR <ups_name> <key> "<value>"` lines for the
/// requested UPS are accepted, so the parser works for any UPS name (not just `ups`).
pub fn parse_list_vars(response: &str, ups_name: &str) -> UpsData {
    let mut data = UpsData::default();

    for line in response.lines() {
        let words = tokenize(line);
        if words.len() < 4 || words[0] != "VAR" || words[1] != ups_name {
            continue;
        }
        let key = words[2].as_str();
        let value = words[3].trim();
        let num = || value.parse::<f64>().ok().filter(|v| v.is_finite());

        match key {
            "battery.charge" => data.battery_charge = num(),
            "battery.runtime" => data.battery_runtime = num(),
            "battery.voltage" => data.battery_voltage = num(),
            "battery.type" => data.battery_type = Some(value.to_string()),
            "input.voltage" => data.input_voltage = num(),
            "input.voltage.fault" => data.input_voltage_fault = num(),
            "input.frequency" => data.input_frequency = num(),
            "output.voltage" => data.output_voltage = num(),
            "output.voltage.nominal" => data.output_voltage_nominal = num(),
            "output.frequency" => data.output_frequency = num(),
            "output.frequency.nominal" => data.output_frequency_nominal = num(),
            "ups.load" => data.ups_load = num(),
            "ups.status" => data.status = value.to_string(),
            "ups.realpower.nominal" => data.ups_realpower_nominal = num(),
            "ups.mfr" => data.ups_mfr = Some(value.to_string()),
            "ups.model" => data.ups_model = Some(value.to_string()),
            "ups.serial" => data.ups_serial = Some(value.to_string()),
            "ups.firmware" => data.ups_firmware = Some(value.to_string()),
            "ups.type" => data.ups_type = Some(value.to_string()),
            "ups.beeper.status" => data.ups_beeper_status = Some(value.to_string()),
            "driver.name" => data.driver_name = Some(value.to_string()),
            "driver.version" => data.driver_version = Some(value.to_string()),
            "ambient.temperature" => data.ambient_temp = num(),
            "output.current" => data.output_current = num(),
            "battery.current" => data.battery_current = num(),
            "ups.realpower" => data.ups_realpower = num(),
            _ => {
                data.extended_vars.insert(key.to_string(), value.to_string());
            }
        }
    }

    // Some UPS drivers report the manufacturer / model only under device.*
    if data.ups_mfr.is_none() {
        data.ups_mfr = data.extended_vars.get("device.mfr").cloned();
    }
    if data.ups_model.is_none() {
        data.ups_model = data.extended_vars.get("device.model").cloned();
    }
    if data.ups_serial.is_none() {
        data.ups_serial = data.extended_vars.get("device.serial").cloned();
    }
    // If input.frequency is missing, show output.frequency instead of nothing.
    if data.input_frequency.is_none() {
        data.input_frequency = data.output_frequency;
    }

    data.calculate_power();
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN LIST VAR qnapups\n\
VAR qnapups battery.charge \"100\"\n\
VAR qnapups battery.runtime \"1800\"\n\
VAR qnapups input.voltage \"229.0\"\n\
VAR qnapups output.voltage \"230.0\"\n\
VAR qnapups ups.load \"23\"\n\
VAR qnapups ups.status \"OL CHRG\"\n\
VAR qnapups device.mfr \"American Power Conversion\"\n\
VAR qnapups ups.model \"Back-UPS \\\"XS\\\" 1500G\"\n\
VAR other battery.charge \"5\"\n\
END LIST VAR qnapups\n";

    #[test]
    fn parses_any_ups_name() {
        let d = parse_list_vars(SAMPLE, "qnapups");
        assert_eq!(d.battery_charge, Some(100.0));
        assert_eq!(d.battery_runtime, Some(1800.0));
        assert_eq!(d.input_voltage, Some(229.0));
        assert_eq!(d.output_voltage, Some(230.0));
        assert_eq!(d.ups_load, Some(23.0));
        assert_eq!(d.status, "OL CHRG");
        assert_eq!(d.ups_mfr.as_deref(), Some("American Power Conversion"));
        assert_eq!(d.ups_model.as_deref(), Some("Back-UPS \"XS\" 1500G"));
    }

    #[test]
    fn ignores_other_ups_and_bad_numbers() {
        let d = parse_list_vars("VAR qnapups battery.charge \"NaN\"\nVAR x ups.load \"5\"\n", "qnapups");
        assert_eq!(d.battery_charge, None);
        assert_eq!(d.ups_load, None);
    }

    #[test]
    fn detects_errors() {
        assert_eq!(error_code("ERR UNKNOWN-UPS\n").as_deref(), Some("UNKNOWN-UPS"));
        assert_eq!(error_code("ERR ACCESS-DENIED extra\n").as_deref(), Some("ACCESS-DENIED"));
        assert_eq!(error_code("BEGIN LIST VAR ups\n"), None);
        assert_eq!(error_code("ERRX\n"), None);
    }
}
