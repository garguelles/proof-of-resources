use serde::Serialize;
use sysinfo::{System, Cpu};
use std::error::Error;
use std::process::Command;

#[derive(Debug)]
enum PlatformError {
    Unsupported(String),
    CommandFailed(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PlatformError::Unsupported(msg) => write!(f, "Unsupported platform: {}", msg),
            PlatformError::CommandFailed(msg) => write!(f, "Command failed: {}", msg),
        }
    }
}

impl Error for PlatformError {}

#[derive(Serialize)]
struct ResourceConfig {
    name: String,
    description: String,
    network: String,
    #[serde(rename = "type")]
    config_type: String,
    config: Config,
}

#[derive(Serialize)]
struct Config {
    resource: Resource,
}

#[derive(Serialize)]
struct Resource {
    ram: Ram,
    ssd: Ssd,
    gpus: Vec<Gpu>,
    cpu: CpuSpec,
}

#[derive(Serialize)]
struct Ram {
    size: u64,
    #[serde(rename = "type")]
    ram_type: String,
}

#[derive(Serialize)]
struct Ssd {
    size: u64,
    #[serde(rename = "type")]
    ssd_type: String,
}

#[derive(Serialize)]
struct Gpu {
    model: String,
}

#[derive(Serialize)]
struct CpuSpec {
    specs: CpuSpecs,
}

#[derive(Serialize)]
struct CpuSpecs {
    cores: u32,
    clock_rate: u64,
}

fn get_ram_type() -> Result<String, Box<dyn Error>> {
    if !cfg!(target_os = "linux") {
        return Err(Box::new(PlatformError::Unsupported(
            "This application only supports Linux".to_string()
        )));
    }

    let output = Command::new("sudo")
        .args(["dmidecode", "--type", "17"])
        .output()
        .map_err(|e| PlatformError::CommandFailed(format!("Failed to execute dmidecode: {}", e)))?;

    if !output.status.success() {
        return Err(Box::new(PlatformError::CommandFailed(
            "dmidecode command failed. Make sure you have sudo privileges and dmidecode is installed".to_string()
        )));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    for line in output_str.lines() {
        if line.contains("Type:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() > 1 {
                let ram_type = parts[1].trim();
                if ram_type.starts_with("DDR") {
                    return Ok(ram_type.to_string());
                }
            }
        }
    }

    Ok("Unknown".to_string())
}

fn get_system_info() -> Result<ResourceConfig, Box<dyn Error>> {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu();

    let ram_type = get_ram_type()?;
    
    // RAM - Convert from KB to bytes
    let total_memory = sys.total_memory() * 1024;

    // CPU info - Convert MHz to Hz
    let cpu_cores = sys.cpus().len() as u32;
    let cpu_frequency = sys.cpus().first()
        .map(|cpu| cpu.frequency() as u64 * 1_000_000)
        .unwrap_or(0);

    let config = ResourceConfig {
        name: String::from("example"),
        description: String::from("Configuration"),
        network: String::from("dev"),
        config_type: String::from("operator"),
        config: Config {
            resource: Resource {
                ram: Ram {
                    size: total_memory,
                    ram_type,
                },
                ssd: Ssd {
                    size: 1099511627776, // 1TB in bytes
                    ssd_type: String::from("NVMeGen4"),
                },
                gpus: vec![Gpu {
                    model: String::from("rtxA4000"),
                }],
                cpu: CpuSpec {
                    specs: CpuSpecs {
                        cores: cpu_cores,
                        clock_rate: cpu_frequency,
                    },
                },
            },
        },
    };

    Ok(config)
}

fn main() -> Result<(), Box<dyn Error>> {
    let system_info = get_system_info()?;
    
    let json = serde_json::to_string_pretty(&system_info)?;
    println!("{}", json);
    
    Ok(())
}
