use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsmSign {
    Plus,
    Minus,
    No,
    Spare,
}

impl SsmSign {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => SsmSign::Plus,
            0b01 => SsmSign::Minus,
            0b10 => SsmSign::No,
            0b11 => SsmSign::Spare,
            _ => unreachable!(),
        }
    }

    pub fn to_bits(self) -> u8 {
        match self {
            SsmSign::Plus => 0b00,
            SsmSign::Minus => 0b01,
            SsmSign::No => 0b10,
            SsmSign::Spare => 0b11,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SsmSign::Plus => "+",
            SsmSign::Minus => "-",
            SsmSign::No => "±",
            SsmSign::Spare => "N/A",
        }
    }
}

impl fmt::Display for SsmSign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadFormat {
    Bnr,
    Bcd,
    Discrete,
    Maintenance,
    Ack,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentId(pub u16);

impl EquipmentId {
    pub fn new(id: u16) -> Self {
        EquipmentId(id)
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

impl fmt::Display for EquipmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EQ-{:04X}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineeringValue {
    pub value: f64,
    pub unit: String,
    pub display: String,
}

impl fmt::Display for EngineeringValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display)
    }
}
