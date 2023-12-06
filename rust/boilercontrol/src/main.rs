use clap::Parser;
use tokio_modbus::prelude::{Reader, Writer};
use serde::Serialize;

const SERIAL: &str = "/dev/serial0";
const ADDRESS: u8 = 250;

const REAL_SUPPLY_MIN: i32 = 85;
const REAL_SUPPLY_MAX: i32 = 171;
const REAL_ODR_MAX: i32 = 63;
const REAL_ODR_MIN: i32 = -14;
const MAX_TEMP_DELTA: i32 = 6;

#[allow(dead_code)]
enum BoilerField {
    OutputTemp = 0x9106,
    BoilerTargetTemp = 0x9109,
    BoilerStatus = 0x9105,
    OutdoorTemp = 0x9112,
    SupplyMax = 0x9120,
    SupplyMin = 0x9121,
    OdrMax = 0x9122,
    OdrMin = 0x9123,
    BoilerMax = 0x9124,
    BoilerOut1 = 0x9180,
    BoilerIn = 0x9188,
    InputStatus = 0x918B,
    FlueTemp = 0x918C,
    SupplyTemp = 0x918D,
    ReturnTemp = 0x9196,
    ModulationRate = 0x9232,
    OdAdjust = 0x9166,
    MaxRate = 0x9131,
}

#[derive(Debug, Serialize)]
struct BoilerInfo {
    time: String,
    output_temp: u16,
    boiler_target_temp: u16,
    boiler_status: u16,
    outdoor_temp: u16,
    supply_max: u16,
    supply_min: u16,
    odr_max: u16,
    odr_min: u16,
    boiler_max: u16,
    boiler_out_1_temp: u16,
    boiler_in_temp: u16,
    input_status: u16,
    flue_temp_1: u16,
    local_supply_temp: u16,
    local_return_temp: u16,
    boiler_modulation_rate: u16,
    outdoor_temp_adjust: u16,
    max_rate: u16,
}

async fn get_full_boiler_info(ctx: &mut tokio_modbus::client::Context) -> Result<BoilerInfo, Box<dyn std::error::Error>> {
    let output_temp = getreg(ctx, BoilerField::OutputTemp).await?;
    let boiler_target_temp = getreg(ctx, BoilerField::BoilerTargetTemp).await?;
    let boiler_status = getreg(ctx, BoilerField::BoilerStatus).await?;
    let outdoor_temp = getreg(ctx, BoilerField::OutdoorTemp).await?;
    let supply_max = getreg(ctx, BoilerField::SupplyMax).await?;
    let supply_min = getreg(ctx, BoilerField::SupplyMin).await?;
    let odr_max = getreg(ctx, BoilerField::OdrMax).await?;
    let odr_min = getreg(ctx, BoilerField::OdrMin).await?;
    let boiler_max = getreg(ctx, BoilerField::BoilerMax).await?;
    let boiler_out_1_temp = getreg(ctx, BoilerField::BoilerOut1).await?;
    let boiler_in_temp = getreg(ctx, BoilerField::BoilerIn).await?;
    let input_status = getreg(ctx, BoilerField::InputStatus).await?;
    let flue_temp_1 = getreg(ctx, BoilerField::FlueTemp).await?;
    let local_supply_temp = getreg(ctx, BoilerField::SupplyTemp).await?;
    let local_return_temp = getreg(ctx, BoilerField::ReturnTemp).await?;
    let boiler_modulation_rate = getreg(ctx, BoilerField::ModulationRate).await?;
    let outdoor_temp_adjust = getreg(ctx, BoilerField::OdAdjust).await?;
    let max_rate = getreg(ctx, BoilerField::MaxRate).await?;

    Ok(BoilerInfo {
        time: chrono::Local::now().naive_utc().to_string(),
        output_temp,
        boiler_target_temp,
        boiler_status,
        outdoor_temp,
        supply_max,
        supply_min,
        odr_max,
        odr_min,
        boiler_max,
        boiler_out_1_temp,
        boiler_in_temp,
        input_status,
        flue_temp_1,
        local_supply_temp,
        local_return_temp,
        boiler_modulation_rate,
        outdoor_temp_adjust,
        max_rate,
    })
}


impl Into<u16> for BoilerField {
    fn into(self) -> u16 {
        self as u16
    }
}

fn calculate_target_temp(outside_temp: u16) -> u16 {
    let outside_temp = outside_temp as i32;
    if outside_temp > REAL_ODR_MAX {
        return REAL_SUPPLY_MIN as u16;
    }
    if outside_temp < REAL_ODR_MIN {
        return REAL_SUPPLY_MAX as u16;
    }
    let percent = (REAL_ODR_MAX - outside_temp) as f32 / (REAL_ODR_MAX - REAL_ODR_MIN) as f32;
    let target = REAL_SUPPLY_MIN as f32 + percent * (REAL_SUPPLY_MAX - REAL_SUPPLY_MIN) as f32;
    target as u16
}

async fn timeout() {
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
}

#[tokio::main]
async fn main() {
    loop {
        if let Err(e) = control_loop().await {
            eprintln!("Error: {}", e);
        }
        tokio::time::sleep(std::time::Duration::from_millis(30000)).await;
    }
}

async fn getreg(
    ctx: &mut tokio_modbus::client::Context,
    field: BoilerField,
) -> Result<u16, Box<dyn std::error::Error>> {
    tokio::select! {
        _ = timeout() => {
            Err("Timeout".into())
        }
        r = ctx.read_holding_registers(field.into(), 1) => {
            Ok(r?[0])
        }
    }
}

async fn setreg(
    ctx: &mut tokio_modbus::client::Context,
    field: BoilerField,
    value: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::select! {
        _ = timeout() => {
            Err("Timeout".into())
        }
        v = ctx.write_single_register(field.into(), value) => {
            v?;
            Ok(())
        }
    }
}

async fn control_loop() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_serial::SerialStream;

    use tokio_modbus::prelude::*;

    let slave = Slave(ADDRESS);

    let builder = tokio_serial::new(SERIAL, 19200)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .timeout(std::time::Duration::from_millis(1000));
    let port = SerialStream::open(&builder).unwrap();

    let mut ctx = rtu::attach_slave(port, slave);
    // only exit this loop if we have an error and need to restart
    loop {
        let info = get_full_boiler_info(&mut ctx).await?;
        let info_json = serde_json::to_string(&info)?;
        println!("{info_json}");
        if info.boiler_status != 0 {
            let calculated_target = calculate_target_temp(info.outdoor_temp);
            // println!("Calculated target temp: {calculated_target}");
            let new_maxrate = match info.boiler_target_temp {
                0..=110 => 21,
                111..=125 => 21 + (info.boiler_target_temp - 110)*2,
                _ => 96,
            };
            if new_maxrate != info.max_rate {
                // println!("Adjusting maxrate to {new_maxrate}");
                setreg(&mut ctx, BoilerField::MaxRate, new_maxrate).await?;
            }
        } else {
            // SAFETY: Turn the max rate back to normal in case anything goes really wrong
            // so that it's less likely to get stuck this way on a crash.
            let maxrate = getreg(&mut ctx, BoilerField::MaxRate).await?;
            if maxrate != 96 {
                // println!("Adjusting maxrate back to 96 while boiler is off");
                setreg(&mut ctx, BoilerField::MaxRate, 96).await?;
            }
        }
        return Ok(()); // Changed mind. Allowing the other processes to grab serial.
    }
    Ok(())
}
