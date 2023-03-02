use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

pub fn read_u16(buf: &Vec<u8>, index: usize) -> u16 {
    LittleEndian::read_u16(&buf[index..index + 2])
}

pub fn read_u32(buf: &Vec<u8>, index: usize) -> u32 {
    println!("{:02x?}", &buf[index..index + 4]);
    LittleEndian::read_u32(&buf[index..index + 4])
}

pub fn read_i16(buf: &Vec<u8>, index: usize) -> i16 {
    LittleEndian::read_i16(&buf[index..index + 2])
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CellsInfo {
    pub cells: Vec<Cell>,
    pub max_cell_voltage: f32,
    pub min_cell_voltage: f32,
    pub average_cell_voltage: f32,
    pub delta_cell_voltage: f32,
    pub total_voltage: f32,
    pub current: f32,
    pub balancing_current: f32,
    pub power: f32,
    pub t1: f32,
    pub t2: f32,
    pub mos_t: f32,
    pub system_alarm: SystemAlarm,
    // wire_resistance_warning_bitmask: Vec<u8>,
    // battery_voltage: f32,
    // battery_power: f32,
    // charge_current: f32,
    // temp_sensor_1: f32,
    // temp_sensor_2: f32,
    // temo_mos: f32,
    // system_alarm: Vec<u8>,
    // balance_current: f32,
    // balancing_action: u8,
    // state_of_charge_in: u8,
    // capacity_remain: f32,
    // nominal_capacity: f32,
    // cycle_count: f32,
    // total_runtime: f32,
    // charging_switch_enabled: u8,
    // discharging_switch_enabled: u8,
    // balancer_status: u8,
    // uptime: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Cell {
    pub voltage: f32,
    pub resistance: f32,
}

impl Cell {
    pub fn new() -> Result<Cell, String> {
        Ok(Cell {
            voltage: 0.0,
            resistance: 0.0,
        })
    }

    pub fn read_voltage(&mut self, buf: &Vec<u8>, index: usize) {
        let voltage_coef: f32 = 0.001;
        self.voltage = read_u16(buf, index) as f32 * voltage_coef;
    }

    pub fn read_resistance(&mut self, buf: &Vec<u8>, index: usize) {
        let resistance_coef: f32 = 0.001;
        self.resistance = read_u16(buf, index) as f32 * resistance_coef;
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemAlarm {
    pub alarm_code: u16,
    pub message: String,
}

impl SystemAlarm {
    pub fn read(alarm_code: u16) -> Result<SystemAlarm, String> {
        let message = match alarm_code {
            0 => "No alarm".to_owned(),
            1 => "Charge overtemperature".to_owned(),
            2 => "Charge undertemperature".to_owned(),
            8 => "Cell Undervoltage".to_owned(),
            1024 => "Cell count is not equal to settings".to_owned(),
            2048 => "Current sensor anomaly".to_owned(),
            4096 => "Cell Over Voltage".to_owned(),
            5120 => "Cell Over Voltage +".to_owned(),
            1032 => "Cell Undervoltage +".to_owned(),
            _ => "Unknown alarm code".to_owned(),
        };

        Ok(SystemAlarm {
            alarm_code,
            message,
        })
    }
}

impl Default for SystemAlarm {
    fn default() -> Self {
        SystemAlarm {
            alarm_code: 0,
            message: "No alarm".to_owned(),
        }
    }
}

impl CellsInfo {
    pub fn new(buf: Vec<u8>) -> Result<CellsInfo, ()> {
        let mut offset = 16;

        let cells_count = 24;
        let mut cells: Vec<Cell> = Vec::new();
        let mut max_cell_voltage: f32 = -100.0;
        let mut min_cell_voltage: f32 = 100.0;

        let mut i = 0;
        while i < cells_count * 2 {
            let mut cell = Cell::new().unwrap();

            let cell_voltage_index = i + 6;
            cell.read_voltage(&buf, cell_voltage_index);

            let cell_resistance_index = i + 64 + offset;
            cell.read_resistance(&buf, cell_resistance_index);

            if cell.voltage > 0.0 && cell.voltage < min_cell_voltage {
                min_cell_voltage = cell.voltage;
            }

            if cell.voltage > max_cell_voltage {
                max_cell_voltage = cell.voltage;
            }

            cells.push(cell);

            i += 2;
        }

        let average_cell_voltage = read_u16(&buf, 58 + offset) as f32 * 0.001;
        let delta_cell_voltage = read_u16(&buf, 60 + offset) as f32 * 0.001;

        offset = offset * 2;

        let total_voltage = read_u32(&buf, 118 + offset) as f32 * 0.001;
        let current = read_u32(&buf, 126 + offset) as f32 * 0.001;
        let power = total_voltage * current;

        let t1 = read_u16(&buf, 130 + offset) as f32 * 0.1;
        let t2 = read_u16(&buf, 132 + offset) as f32 * 0.1;
        let mos_t = read_u16(&buf, 134 + offset) as f32 * 0.1;

        let balancing_current = read_i16(&buf, 138 + offset) as f32 * 0.001;

        let system_alarm = SystemAlarm::read(read_u16(&buf, 136 + offset)).unwrap();

        let p = CellsInfo {
            cells,
            max_cell_voltage,
            min_cell_voltage,
            average_cell_voltage,
            delta_cell_voltage,
            total_voltage,
            current,
            balancing_current,
            power,
            t1,
            t2,
            mos_t,
            system_alarm,
        };

        return Ok(p);
    }
}

impl Default for CellsInfo {
    fn default() -> Self {
        CellsInfo {
            cells: Vec::new(),
            max_cell_voltage: 0.0,
            min_cell_voltage: 0.0,
            average_cell_voltage: 0.0,
            delta_cell_voltage: 0.0,
            total_voltage: 0.0,
            current: 0.0,
            balancing_current: 0.0,
            power: 0.0,
            t1: 0.0,
            t2: 0.0,
            mos_t: 0.0,
            system_alarm: SystemAlarm::default(),
        }
    }
}
