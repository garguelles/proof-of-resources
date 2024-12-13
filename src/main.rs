use serde::Serialize;
use sysinfo::{System, Cpu};
use std::error::Error;

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

fn get_system_info() -> Result<ResourceConfig, Box<dyn Error>> {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu();

    // Get RAM
    let total_memory = sys.total_memory() * 1024; // Convert from KB to bytes

    // Get CPU info
    let cpu_cores = sys.cpus().len() as u32;
    let cpu_frequency = sys.cpus().first()
        .map(|cpu| cpu.frequency() as u64 * 1_000_000) // Convert MHz to Hz
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
                    ram_type: String::from("DDR4"),
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
    
    // Pretty print the JSON
    let json = serde_json::to_string_pretty(&system_info)?;
    println!("{}", json);
    
    Ok(())
}
