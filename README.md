# Proof of resources CLI

A Rust CLI tool that provides detailed system hardware information in JSON format. The tool currently supports Linux systems and provides information about:

- RAM (size and type)
- Storage devices (size and type - NVMe/SATA)
- GPUs (model information)
- CPU (cores and clock rate)

## Prerequisites

### Required Packages

On Debian/Ubuntu systems, install the following packages:

```bash
sudo apt update
sudo apt install dmidecode pciutils smartmontools nvme-cli
```

For other Linux distributions, use your package manager to install:
- `dmidecode` - for RAM information
- `pciutils` - for GPU detection
- `smartmontools` - for storage device information
- `nvme-cli` - for NVMe device information

### Required Permissions

The tool needs root privileges to access hardware information. Run it using `sudo`.

## Installation

1. Clone the repository:
```bash
git clone https://github.com/garguelles/proof-of-resources
cd proof-of-resources
```

2. Build the project:
```bash
cargo build --release
```

## Usage

Run the tool with sudo privileges:

```bash
sudo cargo run
```

### Example Output

```json
{
  "name": "example",
  "description": "Configuration",
  "network": "dev",
  "type": "operator",
  "config": {
    "resource": {
      "ram": {
        "size": 17179869184,
        "type": "DDR4"
      },
      "ssd": {
        "size": 512110190592,
        "type": "NVMeGen4"
      },
      "gpus": [
        {
          "model": "NVIDIA RTX 3080"
        }
      ],
      "cpu": {
        "specs": {
          "cores": 8,
          "clock_rate": 3600000000
        }
      }
    }
  }
}
```
