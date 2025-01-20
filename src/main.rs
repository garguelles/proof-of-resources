use serde::Serialize;
use std::error::Error;
use std::process::Command;
use sysinfo::{System};
use std::fs::File;
use std::io::Write;
use std::fs;


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

fn check_platform() -> Result<(), Box<dyn Error>> {
    if !cfg!(target_os = "linux") {
        return Err(Box::new(PlatformError::Unsupported(
            "This application only supports Linux".to_string(),
        )));
    }

    return Ok(());
}

fn get_gpu_info() -> Result<Vec<String>, Box<dyn Error>> {
    check_platform()?;

    // First try with lspci
    let output = Command::new("lspci")
        .args(["-v"])
        .output()
        .map_err(|e| PlatformError::CommandFailed(format!("Failed to execute lspci: {}", e)))?;

    if !output.status.success() {
        return Err(Box::new(PlatformError::CommandFailed(
            "lspci command failed".to_string(),
        )));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut gpus = Vec::new();

    // Look for VGA and 3D controller entries
    for line in output_str.lines() {
        if line.contains("VGA") || line.contains("3D controller") {
            if line.contains("NVIDIA") {
                // Try nvidia-smi for more detailed info
                if let Ok(nvidia_output) = Command::new("nvidia-smi")
                    .args(["--query-gpu=gpu_name", "--format=csv,noheader"])
                    .output()
                {
                    let nvidia_str = String::from_utf8_lossy(&nvidia_output.stdout);
                    if nvidia_output.status.success() && !nvidia_str.trim().is_empty() {
                        gpus.push(nvidia_str.trim().to_string());
                        continue;
                    }
                }
            }

            // Fallback to lspci output if nvidia-smi fails or for other GPUs
            let gpu_model = line
                .split(':')
                .nth(2)
                .unwrap_or("Unknown GPU")
                .trim()
                .to_string();
            gpus.push(gpu_model);
        }
    }

    if gpus.is_empty() {
        gpus.push("Unknown GPU".to_string());
    }

    Ok(gpus)
}

fn get_ram_type() -> Result<String, Box<dyn Error>> {
    check_platform()?;

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

fn get_storage_info() -> Result<(u64, String), Box<dyn Error>> {
    check_platform()?;

    // Use lsblk with size information
    let output = Command::new("lsblk")
        .args(["-d", "-o", "NAME,TYPE,SIZE,TRAN", "--bytes"]) // --bytes for exact size
        .output()
        .map_err(|e| PlatformError::CommandFailed(format!("Failed to execute lsblk: {}", e)))?;

    if !output.status.success() {
        return Err(Box::new(PlatformError::CommandFailed(
            "lsblk command failed".to_string(),
        )));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut largest_device_size = 0u64;
    let mut storage_type = String::from("Unknown");

    // Try to find NVMe devices first
    for line in output_str.lines().skip(1) {
        // skip header line
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let device_name = parts[0];
            let size_str = parts[2];
            let size = size_str.parse::<u64>().unwrap_or(0);

            if size > largest_device_size {
                largest_device_size = size;

                if device_name.starts_with("nvme") {
                    // Try to get NVMe generation
                    let nvme_info = Command::new("sudo").args(["nvme", "list"]).output();

                    if let Ok(nvme_output) = nvme_info {
                        let nvme_str = String::from_utf8_lossy(&nvme_output.stdout);
                        if nvme_str.contains("PCIe 4.0") {
                            storage_type = "NVMeGen4".to_string();
                        } else if nvme_str.contains("PCIe 3.0") {
                            storage_type = "NVMeGen3".to_string();
                        } else {
                            storage_type = "NVMe".to_string();
                        }
                    }
                } else if parts.get(3).map_or(false, |&t| t == "sata") {
                    // Check if it's an SSD for SATA devices
                    let smart_info = Command::new("sudo")
                        .args(["smartctl", "-i", &format!("/dev/{}", device_name)])
                        .output();

                    if let Ok(smart_output) = smart_info {
                        let smart_str = String::from_utf8_lossy(&smart_output.stdout);
                        if smart_str.contains("Solid State Device") {
                            storage_type = "SATA SSD".to_string();
                        } else {
                            storage_type = "SATA HDD".to_string();
                        }
                    }
                }
            }
        }
    }

    if largest_device_size == 0 {
        return Err(Box::new(PlatformError::CommandFailed(
            "Could not determine storage size".to_string(),
        )));
    }

    Ok((largest_device_size, storage_type))
}

fn get_system_info() -> Result<ResourceConfig, Box<dyn Error>> {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu();

    let ram_type = get_ram_type()?;
    let (storage_size, storage_type) = get_storage_info()?;
    let gpu_models = get_gpu_info()?;

    // CPU info - Convert MHz to Hz
    let cpu_cores = sys.cpus().len() as u32;
    let cpu_frequency = sys
        .cpus()
        .first()
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
                    size: sys.total_memory(),
                    ram_type,
                },
                ssd: Ssd {
                    size: storage_size, // 1TB in bytes
                    ssd_type: storage_type,
                },
                gpus: gpu_models.into_iter().map(|model| Gpu { model }).collect(),
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

    // Create out directory if it doesn't exist
    fs::create_dir_all("out")?;

    // Write to file in the out directory
    let file_path = "out/system_info.json";
    let mut file = File::create(file_path)?;
    file.write_all(json.as_bytes())?;
    println!("System information has been saved to {}", file_path);

    Ok(())
}
