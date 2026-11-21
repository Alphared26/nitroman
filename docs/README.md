# NitroMan: Technical Documentation & User Guide

NitroMan adalah aplikasi utilitas desktop terpadu yang menggantikan Acer Care Center (ACC) dan NitroSense untuk laptop **Acer Nitro AN515-57 (Windows 11, BIOS 1.17)**. Seluruh aplikasi ditulis dalam **Rust murni** dengan antarmuka grafis berbasis **Tauri v2**, tanpa runtime Electron, tanpa background service bloatware, dan tanpa koneksi cloud.

Komunikasi ke Embedded Controller (EC) hardware dilakukan langsung melalui namespace **WMI `root\WMI`** dengan memanggil class yang sama persis seperti yang digunakan driver resmi Acer:

| WMI Class | Tanggung Jawab |
| :--- | :--- |
| `AcerGamingFunction` | Kipas, profil performa, RGB keyboard, telemetri suhu/RPM |
| `BatteryControl` | Pembatas pengisian baterai 80% (Conservation Mode) |
| `AcerGenericMethod` | Pengaturan BIOS/EC generik (`SetFunction`, `GetFunction`) |

Detail format register (opcode `UInt64`) tiap metode WMI didokumentasikan lengkap di [`wmi-protocol.md`](./wmi-protocol.md).

---

## 1. Deskripsi

- **Nama:** NitroMan
- **Target:** Acer Nitro AN515-57 (Windows 11, BIOS 1.17)
- **Teknologi:** Rust + Tauri v2 (frontend HTML + Tailwind CSS + Alpine.js)
- **Akses hardware:** crate `windows` / `wmi` ke namespace `root\WMI`
- **Hak akses:** Administrator (elevated) diperlukan untuk hampir seluruh operasi hardware

Workspace Rust terdiri dari tiga crate:

| Crate | Peran |
| :--- | :--- |
| `nitroman-core` | Library WMI + akses hardware (EC) |
| `nitroman-cli` | CLI diagnostik (`nitroman-cli.exe`) |
| `nitroman-gui` | Desktop app Tauri v2 (`nitroman-gui.exe`) |

> **Catatan Privilege:** Seluruh metode WMI di bawah `root\WMI` memerlukan hak Administrator. NitroMan dirancang dijalankan lewat Windows Task Scheduler dengan RunLevel Highest agar tidak memunculkan prompt UAC setiap kali boot (lihat Bagian 5).

---

## 2. Fitur Utama

- **Telemetri realtime** - Suhu CPU/GPU (derajat C) dan RPM kipas CPU/GPU via `GetGamingSysInfo`.
- **Pengaturan profil termal** - `Quiet`, `Balanced`, `Performance` via `SetGamingProfile`.
- **Fan control** - Mode `Auto`, `Custom` (persentase CPU/GPU manual 0-100%), dan `CoolBoost` (menaikkan threshold RPM hingga ~5500 RPM saat beban tinggi). Mode `max` memaksa 100%.
- **4-Zone RGB keyboard backlight** - Warna per zona (1-4) format hex + efek `Static`, `Breathing`, `Wave`, `Off`.
- **Battery Care 80% limit** - Conservation Mode via `BatteryControl.SetBatteryHealthControl` untuk memperpanjang umur baterai.
- **System tune up / disk clean** - Membersihkan Recycle Bin dan isi `%TEMP%` via PowerShell (`Clear-RecycleBin`, `Remove-Item`).
- **System tray minimization** - Berjalan di tray saat ditutup; flyout popup ringkas untuk profile switcher, toggle baterai, CoolBoost, dan ringkasan suhu/RPM.
- **Windows Task Scheduler autostart** - Task `NitroMan` dengan RunLevel Highest saat logon, tanpa popup UAC.

> **Yang sengaja tidak diimplementasikan:** Overclocking (firmware AN515-57 mengunci multiplier, `CPU_OC_Support = 0`), battery slider arbitrary 50-100% (EC hanya mendukung batas diskrit 80%), LiveUpdate/Support/Planet9 (backend cloud Acer), RGB per-key (hardware AN515-57 bertipe 4-Zone).

---

## 3. Perbandingan Performa

| Metrik | NitroMan | Acer Care Center + NitroSense |
| :--- | :--- | :--- |
| RAM saat idle | 40-60 MB | 300-600 MB |
| Startup time | < 0.5 detik | 5-8 detik |
| Ukuran binary | ~10 MB | Ratusan MB (dengan runtime .NET) |
| Dependency | Pure Rust + Tauri native WMI | Background service bloatware (`PSSvc.exe`, `ACCSvc.exe`) |
| Koneksi internet | Tidak diperlukan | Diperlukan untuk LiveUpdate/Support |

Penghematan RAM idle mencapai **hingga 90%**. Tidak ada background service pabrik Acer yang perlu berjalan; NitroMan cukup satu proses ringan dengan polling sensor periodik.

---

## 4. Panduan CLI (`nitroman-cli.exe`)

Bantuan bawaan:

```text
NitroMan Hardware Control CLI
Target: Acer Nitro AN515-57 (Windows 11, BIOS 1.17)

Usage: nitroman-cli <command> [arguments]

Commands:
  status                        Read realtime sensor telemetry (temps, RPMs, battery)
  profile <quiet|balanced|perf> Set thermal profile
  fan auto                      Set fan mode to automatic curve
  fan max                       Set fans to 100% full speed
  fan custom <cpu_pct> <gpu_pct> Set manual fan speed percentages (0-100)
  battery <on|off>              Toggle 80% battery health conservation limiter
  rgb <hex1> <hex2> <hex3> <hex4> Set 4-zone keyboard backlight colors
  rgb-effect <static|breathing|wave|off> Set keyboard lighting effect
  clean                         Perform temporary disk cleanup
  config                        Display active saved configuration
```

> **Hak Administrator:** Hampir seluruh command (`profile`, `fan`, `battery`, `rgb`, `rgb-effect`, `clean`) memanggil WMI `root\WMI` yang mewajibkan proses elevated. Jalankan dari PowerShell/CMD **Run as Administrator**. Hanya `status` (baca) dan `config` yang bisa berjalan dari user standar, dan sebagian besar field akan `N/A` bila tidak elevated. Status hak terlihat di baris `Elevated Rights` pada output `status`.

### 4.1. `status` - Telemetri Sensor

Membaca suhu CPU/GPU, RPM kipas, persentase baterai, serta konfigurasi aktif.

```text
PS> nitroman-cli status
========================================================
NITROMAN SENSOR TELEMETRY
========================================================
  CPU Temperature  : 55.0 C
  GPU Temperature  : 48.5 C
  CPU Fan Speed    : 2100 RPM
  GPU Fan Speed    : 1980 RPM
  Battery Charge   : 80 %
  Battery 80% Cap  : Enabled
  Thermal Profile  : Balanced
  Fan Mode         : Auto
  Elevated Rights  : Yes (Administrator)
========================================================
```

Nilai `N/A` menandakan sensor tersebut tidak terbaca (biasanya karena proses tidak elevated atau class WMI tidak tersedia).

### 4.2. `profile` - Profil Termal

```text
PS> nitroman-cli profile quiet
Thermal profile successfully switched to Quiet
```

```text
PS> nitroman-cli profile perf
Thermal profile successfully switched to Performance
```

Argumen yang diterima: `quiet`, `balanced`, `perf` (atau `performance`). Profil aktif disimpan ke config.

### 4.3. `fan` - Kontrol Kipas

Mode otomatis (kurva pabrik EC):

```text
PS> nitroman-cli fan auto
Fan mode set to Auto (factory EC curve)
```

Mode 100% penuh:

```text
PS> nitroman-cli fan max
Fan mode set to Maximum (100% full speed)
```

Mode manual (CPU dan GPU masing-masing 0-100):

```text
PS> nitroman-cli fan custom 50 50
Manual fan speeds applied: CPU 50%, GPU 50%
```

```text
PS> nitroman-cli fan custom 80 60
Manual fan speeds applied: CPU 80%, GPU 60%
```

Catatan: `fan custom` mengirim `SetGamingFanBehavior` mode Custom lalu `SetGamingFanSpeed`. Argumen di luar rentang 0-100 akan di-parse ke nilai default 50.

### 4.4. `battery` - Battery Care 80%

```text
PS> nitroman-cli battery on
Battery 80% charge limiter: Enabled (Conservation Mode active)
```

```text
PS> nitroman-cli battery off
Battery 80% charge limiter: Disabled (Charges to 100%)
```

Menerima `on` / `1` / `enable` untuk mengaktifkan, `off` / `0` / `disable` untuk mematikan.

### 4.5. `rgb` - Warna 4 Zona

Mengatur warna Zone 1 sampai Zone 4, masing-masing satu argumen hex (6 digit, `#` opsional).

```text
PS> nitroman-cli rgb #E4002B #FF4400 #00A3FF #9900FF
4-zone RGB keyboard colors successfully updated
```

Warna bisa ditulis tanpa `#`:

```text
PS> nitroman-cli rgb E4002B 00FF00 0000FF FFFFFF
4-zone RGB keyboard colors successfully updated
```

Format hex salah akan menghasilkan error per zona:

```text
PS> nitroman-cli rgb #E4002B #12 #00A3FF #9900FF
Invalid hex color for Zone 2: #12
```

### 4.6. `rgb-effect` - Efek Pencahayaan

```text
PS> nitroman-cli rgb-effect breathing
Keyboard lighting effect set to Breathing
```

```text
PS> nitroman-cli rgb-effect off
Keyboard lighting effect set to Off
```

Nilai yang diterima: `static`, `breathing`, `wave`, `off`.

### 4.7. `clean` - Disk Cleanup

Menjalankan pembersihan sementara Windows (mengosongkan Recycle Bin dan menghapus isi `%TEMP%`):

```text
PS> nitroman-cli clean
Executing temporary Windows disk cleanup...
Cleanup finished successfully.
```

Pembersihan dilakukan via PowerShell: `Clear-RecycleBin -Force` dan `Remove-Item "$env:TEMP\*" -Recurse -Force` (kegagalan per item diabaikan secara senyap).

### 4.8. `config` - Konfigurasi Aktif

Menampilkan konfigurasi tersimpan dalam format JSON:

```text
PS> nitroman-cli config
{
  "battery_80_limit": true,
  "thermal_profile": "Balanced",
  "fan_mode": "Auto",
  "cpu_fan_manual": 50,
  "gpu_fan_manual": 50,
  "rgb": {
    "zones": [
      { "r": 228, "g": 0, "b": 43 },
      ...
    ],
    "effect": "Breathing"
  }
}
```

---

## 5. Instalasi Autostart via Task Scheduler

Agar NitroMan berjalan saat Windows menyala tanpa memunculkan prompt UAC, gunakan Task Scheduler dengan **RunLevel Highest**.

### 5.1. Mendaftarkan Task (`register_task.ps1`)

Skrip ini membuat Scheduled Task bernama `NitroMan` yang dipicu **saat user logon** dan dijalankan dengan hak tertinggi. Task juga diatur agar tetap jalan walau memakai baterai dan tanpa batas waktu eksekusi (`ExecutionTimeLimit 0`).

Parameter opsional:

- `-BinaryPath` - Path ke executable. Default: `D:\Project\HermesAI\nitroman\target\release\nitroman-gui.exe`

Jalankan dari **PowerShell as Administrator**:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\register_task.ps1
```

Dengan path binary kustom:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\register_task.ps1 -BinaryPath "C:\Tools\NitroMan\nitroman-gui.exe"
```

Output yang diharapkan:

```text
Configuring Task Scheduler for NitroMan...
Target executable: D:\Project\HermesAI\nitroman\target\release\nitroman-gui.exe
Successfully registered elevated task 'NitroMan'.
```

Skrip otomatis menghapus task `NitroMan` lama (jika ada) sebelum mendaftarkan ulang.

### 5.2. Menghapus Task (`unregister_task.ps1`)

Jalankan dari **PowerShell as Administrator**:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\unregister_task.ps1
```

Output yang diharapkan:

```text
Removing 'NitroMan' from Task Scheduler...
Successfully unregistered 'NitroMan'.
```

Bila task tidak pernah terdaftar:

```text
Removing 'NitroMan' from Task Scheduler...
Task was not registered or could not be removed: <pesan>
```

---

## 6. Build dari Source

Build seluruh workspace dalam mode release:

```bash
cargo build --release -p nitroman-gui -p nitroman-cli
```

Lokasi binary hasil build:

```text
D:\Project\HermesAI\nitroman\target\release\nitroman-gui.exe
D:\Project\HermesAI\nitroman\target\release\nitroman-cli.exe
```

Setelah build, daftarkan autostart GUI lewat skrip di Bagian 5, atau gunakan CLI langsung dari PowerShell elevated sesuai Bagian 4.