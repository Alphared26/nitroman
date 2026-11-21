# NitroMan — Hardware Control Center for Acer Nitro AN515-57

NitroMan is a high-performance, lightweight hardware management center and in-game telemetry overlay for Acer Nitro laptops (specifically tuned for Acer Nitro 5 AN515-57 with Intel Core i7-11800H and NVIDIA RTX 3050 Ti Laptop GPU).

Built with Rust and Tauri (WebView2), NitroMan completely replaces heavy, slow OEM utilities (Acer NitroSense and Acer Care Center) with a fast (~18 MB RAM), bloatware-free, native desktop experience.

---

## Key Capabilities

### 1. Hardware Fan & Thermal Management
- Dual-fan RPM telemetry for CPU and GPU blower fans.
- Modes: Firmware Auto, Max Ceiling (5800 RPM), and Custom PWM manual curves.
- CoolBoost toggle (+500 RPM thermal ceiling headroom during gaming loads).
- Thermal Profiles: Quiet, Balanced, and Performance modes.

### 2. 4-Zone RGB Keyboard Lighting Engine
- Direct hardware EC control via Command 28, 27, and 17.
- Individual color control across 4 keyboard zones (Zone 1 WASD, Zone 2 Mid-Left, Zone 3 Mid-Right & Arrows, Zone 4 Numpad & Nitro Key).
- 12 Hardware lighting modes: Static, Breathing, Wave, Zoom, Shifting, Neon, Strobe, Chase, Police, Thermal Reactive, Typing Wave, and Off.
- Fine-grained controls: Brightness slider, speed multiplier, direction selector, and 30-second backlight auto-sleep timeout.

### 3. Battery Conservation & Health Control
- 80% Battery Charge Limiter (Conservation Mode) to preserve lithium-ion battery lifespan when plugged into AC power.
- Live battery health, cycle count, and charging state telemetry.

### 4. Real-time In-game Telemetry OSD (HUD Overlay)
- Frameless, semi-transparent, draggable, and resizable on-screen display widget.
- Real-time CPU clock frequency (via Windows PDH counter), temperatures, load percentage, power draw in Watts.
- GPU temperature, core clock, VRAM usage, and in-game FPS counter (via Windows Graphics DWM capture).
- Real-time disk drive temperatures (NVMe SSD 1, NVMe SSD 2, and SATA HDD).
- Dynamic in-widget settings panel to toggle individual metrics and adjust HUD scale.

### 5. Avionics System Maintenance
- One-click TRIM optimization across all NVMe and SATA drives.
- Standby memory cache flusher (`EmptyWorkingSet`).
- System temporary file purge.

---

## Standalone Windows Installer

NitroMan provides a single-file installer built with Inno Setup 6:
`dist/NitroMan-Setup-v1.0.0.exe` (~17.3 MB)

### Installer Features:
- Fully bundles required low-level background services (`NitroSense Service` / `PSSvc.exe` and Care Center WMI utilities) so users can safely uninstall official Acer applications without losing physical hardware control.
- Automatically registers and starts `PSSvc` service if missing.
- Automatically creates an elevated Windows Task Scheduler task (`RunLevel=HighestAvailable`) so NitroMan starts automatically at Windows logon without UAC confirmation dialogs.
- Installs desktop and Start Menu shortcuts with the official NitroMan crest shield icon.
- Clean uninstallation via Windows Settings / Control Panel.

---

## Repository Structure

- `nitroman-core/`: Hardware abstraction layer, EC named pipe IPC driver, WMI controller, NVML integration, PDH clock sensor, and storage health telemetry.
- `nitroman-gui/`: Tauri application backend and frontend UI (Dashboard, Fans, RGB, Specs, and HUD overlay).
- `nitroman-cli/`: Standalone command-line interface for scripting and terminal operations.
- `installer/`: Inno Setup 6 compilation script and bundled service dependencies.
- `dist/`: Compiled production binaries, setup installer, and portable zip package.
- `scripts/`: Task Scheduler registration and unregistration PowerShell scripts.
- `docs/`: Technical plans, architecture specifications, and verification checklists.

---

## Requirements & Compatibility

- Target Device: Acer Nitro 5 (Model AN515-57 and compatible Acer Nitro / Predator series).
- Operating System: Windows 10 / Windows 11 (64-bit).
- Privileges: Administrator rights required for low-level EC hardware access.
- Runtime: Microsoft Edge WebView2 (preinstalled on Windows 10/11).
