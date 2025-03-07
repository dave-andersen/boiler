use clap::Parser;
use pid::Pid;
use reqwest::Client;
use serde::Serialize;
use std::io::Write;
use std::sync::Mutex;
use std::sync::OnceLock;
use tokio_modbus::prelude::{Reader, Writer};

const REAL_SUPPLY_MIN: i32 = 85;
const REAL_SUPPLY_MAX: i32 = 171;
const REAL_ODR_MAX: i32 = 63;
const REAL_ODR_MIN: i32 = -14;
const MAX_TEMP_DELTA: i32 = 6;

#[derive(Parser, Debug)]
struct Opts {
    /// Serial device to use
    #[arg(short, long, default_value = "/dev/serial0")]
    serial: String,

    /// Serial port speed
    #[arg(long, default_value = "19200")]
    serial_speed: u32,

    /// Modbus address of the boiler
    #[arg(short, long, default_value = "250")]
    address: u8,

    /// Enable control of the boiler
    #[arg(short, long)]
    control: bool,

    /// Temperature sensor IP address
    #[arg(long, default_value = "192.168.49.11")]
    sensor_ip: String,

    /// Override min max modulation rate
    #[arg(long)]
    override_min_max: Option<u16>,

    /// Emit verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Logging host:port
    #[arg(long)]
    loghost: String,
}

// Modbus registers, specific to Weil-Mclain Evergreen
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
    indoor_temp: Option<f32>,
}

async fn get_full_boiler_info(
    ctx: &mut tokio_modbus::client::Context,
) -> Result<BoilerInfo, Box<dyn std::error::Error>> {
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
        indoor_temp: None,
    })
}

impl From<BoilerField> for u16 {
    fn from(val: BoilerField) -> Self {
        val as u16
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
    let opts: Opts = Opts::parse();
    let loghost = opts.loghost.clone();
    let socket = if !opts.loghost.is_empty() {
        Some({
            let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();
            socket.connect(&loghost).await.unwrap();
            socket
        })
    } else {
        None
    };
    loop {
        let logsocket = socket.as_ref();
        let sleep_duration = match control_loop(&opts, logsocket).await {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Error: {}", e);
                30000
            }
        };
        tokio::time::sleep(std::time::Duration::from_millis(sleep_duration)).await;
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

#[derive(serde::Deserialize)]
struct TempResponse {
    temp: f32, // in C
}

async fn get_indoor_temp(opts: &Opts) -> Result<f32, Box<dyn std::error::Error>> {
    let client = Client::new();
    // Build a request with a 1 second timeout
    let req = client
        .get(format!("http://{}/", opts.sensor_ip))
        .timeout(std::time::Duration::from_secs(1));
    let resp = req.send().await?;
    // unmarshal from json TempResponse
    let resp: TempResponse = resp.json().await?;
    Ok(resp.temp)
}

static pid_cell: OnceLock<Mutex<Pid<f32>>> = OnceLock::new();

async fn control_loop(
    opts: &Opts,
    logsocket: Option<&tokio::net::UdpSocket>,
) -> Result<(u64), Box<dyn std::error::Error>> {
    use tokio_serial::SerialStream;

    use tokio_modbus::prelude::*;

    let slave = Slave(opts.address);

    let builder = tokio_serial::new(&opts.serial, opts.serial_speed)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .timeout(std::time::Duration::from_millis(1000));
    let port = SerialStream::open(&builder).unwrap();

    let mut ctx = rtu::attach_slave(port, slave);

    // A static PID controller that hangs out forever
    {
        let _pid = pid_cell.get_or_init(|| {
            let mut p = Pid::<f32>::new(19.875, 80.0);
            p.p(1.0, 100.0).i(1.0, 100.0).d(1.0, 100.0);
            Mutex::new(p)
        });
    }

    loop {
        let indoor_temp = get_indoor_temp(&opts).await.ok();
        let mut info = get_full_boiler_info(&mut ctx).await?;
        info.indoor_temp = indoor_temp;
        let info_json = serde_json::to_string(&info)?;

        // Either send to UDP socket or log to file
        match logsocket {
            Some(socket) => {
                // Send via UDP to the log host
                socket.send(info_json.as_bytes()).await?;
            }
            None => {
                // Log this to log.json as before
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open("log.json")
                {
                    file.write_all(info_json.as_bytes())?;
                    file.write_all(b"\n")?;
                }
            }
        }

        if info.boiler_status != 0 {
            let target_temp = match info.boiler_status {
                131 => info.boiler_target_temp,
                _ => calculate_target_temp(info.outdoor_temp),
            };
            let mut new_maxrate = match target_temp {
                0..=110 => 21,
                111..=160 => 21 + ((info.boiler_target_temp - 110) as f32 * 1.0) as u16,
                _ => 96,
            };
            // Target-based control
            // Current temp behavior:
            // If indoor temp reaches 20.4375, thermostat seems to shut off.
            // It can go as low as 19.6875 but probably turns on just a hair before that.
            // So as we get close, back off the target temp by adjusting the outdoor reset offset.
            let temp_thresh = 19.6;
            if info.indoor_temp.is_some_and(|t| t >= temp_thresh) {
                let t = info.indoor_temp.unwrap();
                if opts.verbose {
                    println!("Plan: Invoke PID controller");
                }
                let control_output = if new_maxrate > 24 {
                    pid_cell
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .next_control_output(t)
                        .output
                } else {
                    pid_cell
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .reset_integral_term();
                    new_maxrate as f32 - 21.0
                };
                let mut try_maxrate = (21.0 + control_output).floor() as u16;
                try_maxrate = std::cmp::max(21, try_maxrate);
                try_maxrate = std::cmp::min(try_maxrate, new_maxrate);
                println!("PID reult: {control_output}, setting maxrate to {try_maxrate}");
                new_maxrate = try_maxrate;
            }
            if let Some(override_val) = opts.override_min_max {
                if opts.verbose {
                    println!("Plan: override min maxrate to {override_val}");
                }
                new_maxrate = override_val;
            }
            if new_maxrate != info.max_rate {
                if opts.verbose {
                    println!("Plan: set maxrate to {new_maxrate}");
                }
                if opts.control {
                    setreg(&mut ctx, BoilerField::MaxRate, new_maxrate).await?;
                }
            }
        } else {
            // SAFETY: Turn the max rate back to normal in case anything goes really wrong
            // so that it's less likely to get stuck this way on a crash.
            pid_cell
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .reset_integral_term();

            if info.max_rate < 50 {
                if opts.verbose {
                    println!(
                        "Plan: set max_rate from {} to 50 because boiler is off",
                        info.max_rate
                    );
                }
                if opts.control {
                    setreg(&mut ctx, BoilerField::MaxRate, 50).await?;
                }
            }
            if info.outdoor_temp_adjust != 0 {
                if opts.verbose {
                    println!("Plan: set outdoor temp adjust to 0 because boiler is off");
                }
                if opts.control {
                    let r = setreg(&mut ctx, BoilerField::OdAdjust, 0).await;
                    if let Err(e) = r {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        }
        if info.indoor_temp.is_some_and(|t| t > 72.0) || info.outdoor_temp > 70 {
            return Ok(300 * 1000); // sleep for 5 minutes
        }
        return Ok(30 * 1000); // Changed mind. Allowing the other processes to grab serial.
    }
    Ok(30000)
}
