use crate::core::types::EngineeringValue;

#[derive(Debug, Clone)]
pub struct DiscreteBit {
    pub position: u8,
    pub name: String,
    pub active_high: bool,
}

impl DiscreteBit {
    pub fn new(position: u8, name: &str) -> Self {
        DiscreteBit {
            position,
            name: name.to_string(),
            active_high: true,
        }
    }

    pub fn active_low(mut self) -> Self {
        self.active_high = false;
        self
    }
}

#[derive(Debug, Clone)]
pub struct DiscreteWordDef {
    pub name: String,
    pub bits: Vec<DiscreteBit>,
}

impl DiscreteWordDef {
    pub fn new(name: &str, bits: Vec<DiscreteBit>) -> Self {
        DiscreteWordDef {
            name: name.to_string(),
            bits,
        }
    }
}

pub fn decode_discrete(data: u32, def: &DiscreteWordDef) -> EngineeringValue {
    let mut active_bits: Vec<String> = Vec::new();

    for bit in &def.bits {
        let is_set = ((data >> bit.position) & 0x1) == 1;
        let is_active = if bit.active_high { is_set } else { !is_set };

        if is_active {
            active_bits.push(bit.name.clone());
        }
    }

    let display = if active_bits.is_empty() {
        format!("{}: NO BITS", def.name)
    } else {
            format!("{}: [{}]", def.name, active_bits.join(" | "))
        };

    EngineeringValue {
        value: data as f64,
        unit: "bitmask".to_string(),
        display,
    }
}

pub fn decode_gear_discrete(data: u32) -> EngineeringValue {
    let def = DiscreteWordDef::new("LANDING_GEAR", vec![
        DiscreteBit::new(0, "NOSE_GEAR_DOWN"),
        DiscreteBit::new(1, "NOSE_GEAR_UP"),
        DiscreteBit::new(2, "NOSE_GEAR_LOCK"),
        DiscreteBit::new(3, "LEFT_GEAR_DOWN"),
        DiscreteBit::new(4, "LEFT_GEAR_UP"),
        DiscreteBit::new(5, "LEFT_GEAR_LOCK"),
        DiscreteBit::new(6, "RIGHT_GEAR_DOWN"),
        DiscreteBit::new(7, "RIGHT_GEAR_UP"),
        DiscreteBit::new(8, "RIGHT_GEAR_LOCK"),
        DiscreteBit::new(9, "GEAR_IN_TRANSIT"),
        DiscreteBit::new(10, "BRAKES_APPLIED"),
        DiscreteBit::new(11, "ANTISKID_ON"),
        DiscreteBit::new(12, "GEAR_LEVER_DOWN"),
        DiscreteBit::new(13, "GEAR_LEVER_UP"),
        DiscreteBit::new(14, "WHEEL_WEIGHT_ON"),
        DiscreteBit::new(15, "WHEEL_WEIGHT_OFF"),
        DiscreteBit::new(16, "DOOR_OPEN"),
        DiscreteBit::new(17, "DOOR_CLOSED"),
        DiscreteBit::new(18, "HYD_PRESS_OK"),
    ]);
    decode_discrete(data, &def)
}

pub fn decode_door_status(data: u32) -> EngineeringValue {
    let def = DiscreteWordDef::new("DOOR_STATUS", vec![
        DiscreteBit::new(0, "L1_DOOR_OPEN"),
        DiscreteBit::new(1, "L1_DOOR_CLOSED"),
        DiscreteBit::new(2, "L1_DOOR_LOCKED"),
        DiscreteBit::new(3, "R1_DOOR_OPEN"),
        DiscreteBit::new(4, "R1_DOOR_CLOSED"),
        DiscreteBit::new(5, "R1_DOOR_LOCKED"),
        DiscreteBit::new(6, "L2_DOOR_OPEN"),
        DiscreteBit::new(7, "L2_DOOR_CLOSED"),
        DiscreteBit::new(8, "L2_DOOR_LOCKED"),
        DiscreteBit::new(9, "R2_DOOR_OPEN"),
        DiscreteBit::new(10, "R2_DOOR_CLOSED"),
        DiscreteBit::new(11, "R2_DOOR_LOCKED"),
        DiscreteBit::new(12, "FWD_CARGO_OPEN"),
        DiscreteBit::new(13, "FWD_CARGO_CLOSED"),
        DiscreteBit::new(14, "AFT_CARGO_OPEN"),
        DiscreteBit::new(15, "AFT_CARGO_CLOSED"),
        DiscreteBit::new(16, "EMER_EXIT_ARMED"),
        DiscreteBit::new(17, "DOOR_SLIDE_ARMED"),
        DiscreteBit::new(18, "PASSENGER_DOORS_INHIBIT"),
    ]);
    decode_discrete(data, &def)
}

pub fn decode_anti_ice(data: u32) -> EngineeringValue {
    let def = DiscreteWordDef::new("ANTI_ICE", vec![
        DiscreteBit::new(0, "WING_L_ENG1_AI_ON"),
        DiscreteBit::new(1, "WING_R_ENG2_AI_ON"),
        DiscreteBit::new(2, "ENG1_COWL_AI_ON"),
        DiscreteBit::new(3, "ENG2_COWL_AI_ON"),
        DiscreteBit::new(4, "ENG3_COWL_AI_ON"),
        DiscreteBit::new(5, "ENG4_COWL_AI_ON"),
        DiscreteBit::new(6, "PROBE_PITOT1_ON"),
        DiscreteBit::new(7, "PROBE_PITOT2_ON"),
        DiscreteBit::new(8, "PROBE_AOA_ON"),
        DiscreteBit::new(9, "PROBE_STATIC_ON"),
        DiscreteBit::new(10, "WINDSHIELD_1_HOT"),
        DiscreteBit::new(11, "WINDSHIELD_2_HOT"),
        DiscreteBit::new(12, "TAT_PROBE_HEAT_ON"),
        DiscreteBit::new(13, "ICE_DETECTED"),
        DiscreteBit::new(14, "ICE_DETECTOR_A_ON"),
        DiscreteBit::new(15, "ICE_DETECTOR_B_ON"),
    ]);
    decode_discrete(data, &def)
}

pub fn decode_autopilot_modes(data: u32) -> EngineeringValue {
    let def = DiscreteWordDef::new("A/P MODES", vec![
        DiscreteBit::new(0, "AP1_ENGAGED"),
        DiscreteBit::new(1, "AP2_ENGAGED"),
        DiscreteBit::new(2, "FD1_ON"),
        DiscreteBit::new(3, "AT_ARMED"),
        DiscreteBit::new(4, "AT_ACTIVE"),
        DiscreteBit::new(5, "HDG_MODE"),
        DiscreteBit::new(6, "NAV_MODE"),
        DiscreteBit::new(7, "APP_MODE"),
        DiscreteBit::new(8, "ALT_HOLD"),
        DiscreteBit::new(9, "VS_MODE"),
        DiscreteBit::new(10, "FLCH_MODE"),
        DiscreteBit::new(11, "VNAV_MODE"),
        DiscreteBit::new(12, "LNAV_MODE"),
        DiscreteBit::new(13, "LAND_ARMED"),
        DiscreteBit::new(14, "GPWS_INHIBIT"),
        DiscreteBit::new(15, "TCAS_TA_ON"),
        DiscreteBit::new(16, "AUTOBRAKE_ON"),
        DiscreteBit::new(17, "AUTO_THRUST_LVR"),
        DiscreteBit::new(18, "FLARE_MODE"),
    ]);
    decode_discrete(data, &def)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gear_down_and_locked() {
        let data = (1 << 0) | (1 << 2) | (1 << 3) | (1 << 5) | (1 << 6) | (1 << 8) | (1 << 15);
        let result = decode_gear_discrete(data);
        assert!(result.display.contains("NOSE_GEAR_DOWN"));
        assert!(result.display.contains("LEFT_GEAR_DOWN"));
        assert!(result.display.contains("RIGHT_GEAR_DOWN"));
        assert!(result.display.contains("LOCK"));
        assert!(result.display.contains("WHEEL_WEIGHT_OFF"));
    }

    #[test]
    fn test_gear_up() {
        let data = (1 << 1) | (1 << 4) | (1 << 7) | (1 << 14);
        let result = decode_gear_discrete(data);
        assert!(result.display.contains("NOSE_GEAR_UP"));
        assert!(result.display.contains("WHEEL_WEIGHT_ON"));
    }

    #[test]
    fn test_door_closed() {
        let data = (1 << 1) | (1 << 2) | (1 << 4) | (1 << 5);
        let result = decode_door_status(data);
        assert!(result.display.contains("L1_DOOR_CLOSED"));
        assert!(result.display.contains("L1_DOOR_LOCKED"));
        assert!(result.display.contains("R1_DOOR_CLOSED"));
    }

    #[test]
    fn test_anti_ice_all_on() {
        let data = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 13);
        let result = decode_anti_ice(data);
        assert!(result.display.contains("WING_L_ENG1_AI_ON"));
        assert!(result.display.contains("ICE_DETECTED"));
    }

    #[test]
    fn test_no_bits() {
        let def = DiscreteWordDef::new("TEST", vec![DiscreteBit::new(0, "TEST_BIT")]);
        let result = decode_discrete(0, &def);
        assert!(result.display.contains("NO BITS"));
    }

    #[test]
    fn test_ap_modes_complex() {
        let data = (1 << 0) | (1 << 5) | (1 << 8) | (1 << 11);
        let result = decode_autopilot_modes(data);
        assert!(result.display.contains("AP1_ENGAGED"));
        assert!(result.display.contains("HDG_MODE"));
        assert!(result.display.contains("ALT_HOLD"));
        assert!(result.display.contains("VNAV_MODE"));
    }
}
