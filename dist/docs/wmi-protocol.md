# NitroMan WMI Protocol & Hardware Interface Specification

Dokumentasi hasil Reverse Engineering terhadap service asli Acer (PSSvc.exe, ACCSvc.exe, ACCUtilities.dll, dan TsDotNetLib.dll) untuk laptop **Acer Nitro AN515-57 (Windows 11, BIOS 1.17)**.

Semua interaksi hardware ACPI Embedded Controller (EC) dilakukan melalui namespace WMI: **`root\WMI`**.

---

## 1. Ringkasan WMI Classes

| WMI Class | Tanggung Jawab Utama | Metode Kunci |
| :--- | :--- | :--- |
| `AcerGamingFunction` | Kontrol Kipas, Profil Performa, RGB Keyboard, Telemetri Suhu/RPM | `SetGamingProfile`, `SetGamingFanSpeed`, `SetGamingFanBehavior`, `SetGamingFanTable`, `GetGamingSysInfo`, `SetGamingRgbKb`, `SetGamingLEDColor` |
| `BatteryControl` | Pembatas Pengisian Baterai (80% Battery Care / Conservation Mode) | `SetBatteryHealthControl`, `GetBatteryHealthControlStatus` |
| `AcerGenericMethod` | Pengaturan BIOS/EC Generik | `SetFunction`, `GetFunction` |

> [CATATAN PRIVILEGE] Seluruh metode WMI di bawah `root\WMI` memerlukan hak **Administrator (Elevated)**. Pada NitroMan, proses dijalankan via **Windows Task Scheduler** dengan opsi *Highest Privileges* saat user login agar tidak memunculkan dialog UAC setiap kali boot.

---

## 2. Fan Control & Monitoring (`AcerGamingFunction`)

### 2.1. Membaca Sensor Realtime: `GetGamingSysInfo`
* **Metode:** `AcerGamingFunction.GetGamingSysInfo(gmInput: UInt32, out gmOutput: UInt64)`
* **Input (UInt32):**
  * `0x00000001` : Query Suhu CPU & Fan 1 RPM
  * `0x00000002` : Query Suhu GPU & Fan 2 RPM
* **Output Decoding (UInt64):**
  * Byte 0..1 (Bits 0–15): Kecepatan Kipas aktual dalam RPM (Rotations Per Minute).
  * Byte 2 (Bits 16–23): Suhu aktual dalam °C.
  * Byte 3..7: Reserved / Status bit.

### 2.2. Mengatur Mode Kipas: `SetGamingFanBehavior`
* **Metode:** `AcerGamingFunction.SetGamingFanBehavior(gmInput: UInt64, out gmOutput: UInt32)`
* **Format `gmInput` (UInt64):**
  * `0x00000001_00000000` : **Auto Mode** (Kipas dikontrol otomatis oleh kurva pabrik EC)
  * `0x00000002_00000000` : **Max / Performance Mode** (Kipas dipaksa 100% RPM)
  * `0x00000003_00000000` : **Custom / Manual Mode** (Kipas mengikuti persentase manual / custom curve)

### 2.3. Mengatur Persentase Kipas Manual: `SetGamingFanSpeed`
* **Metode:** `AcerGamingFunction.SetGamingFanSpeed(gmInput: UInt64, out gmOutput: UInt32)`
* Digunakan ketika Fan Behavior disetel ke mode Custom/Manual.
* **Format `gmInput` (UInt64):**
  ```text
  [Bits 63..32] : Reserved (0x00000000)
  [Bits 31..16] : Target GPU Fan Percentage (0 - 100)
  [Bits 15..0]  : Target CPU Fan Percentage (0 - 100)
  ```
  *Contoh:* CPU 50% (`0x32`), GPU 50% (`0x32`) = `0x00000000_00320032` (Desimal: `3276850`).

### 2.4. CoolBoost™ Toggle
* Pengaturan CoolBoost mengontrol apakah EC diizinkan menaikkan *threshold* RPM kipas melebihi batas standar (hingga ~5500 RPM) saat load tinggi:
  * Registry Mapping: `HKLM\SOFTWARE\OEM\NitroSense\FanControl\CoolBoostMode` (DWORD: `1` = Aktif, `0` = Mati).
  * Diimplementasikan bersamaan dengan pengiriman update mode fan ke EC.

---

## 3. Thermal Profile (`AcerGamingFunction`)

### 3.1. Mengatur Profil Performa: `SetGamingProfile`
* **Metode:** `AcerGamingFunction.SetGamingProfile(gmInput: UInt64, out gmOutput: UInt32)`
* **Format `gmInput` (UInt64):**
  * `0x00000000` : **Quiet Mode** (TDP dibatasi, akustik senyap)
  * `0x00000001` : **Balanced Mode** (Standar pabrik Acer)
  * `0x00000002` : **Performance Mode** (PL1/PL2 maksimum, fan lebih agresif)

---

## 4. Baterai: 80% Limit / Conservation Mode (`BatteryControl`)

Acer Care Center mengendalikan proteksi baterai lewat WMI Class **`root\WMI:BatteryControl`**.

### 4.1. Membaca Status Limit Baterai: `GetBatteryHealthControlStatus`
* **Metode:** `BatteryControl.GetBatteryHealthControlStatus(uBatteryNo, uFunctionQuery, ...)`
* **Parameter:**
  * `uBatteryNo` (UInt8): `0x01` (Baterai Utama)
  * `uFunctionQuery` (UInt8): `0x01`
* **Output:**
  * `uFunctionStatus`: Array UInt8 / Bitmask. Bit 0 menandakan status limit:
    * `1` : 80% Battery Health Protection Aktif.
    * `0` : Pengisian Normal hingga 100%.

### 4.2. Mengaktifkan / Menonaktifkan Limit 80%: `SetBatteryHealthControl`
* **Metode:** `BatteryControl.SetBatteryHealthControl(uBatteryNo, uFunctionMask, uFunctionStatus, uReservedIn, ...)`
* **Parameter:**
  * `uBatteryNo` (UInt8): `0x01`
  * `uFunctionMask` (UInt8): `0x01` (Mask untuk fitur Battery Health Limiter)
  * `uFunctionStatus` (UInt8):
    * `0x01` : Aktifkan limit 80% (Conservation Mode)
    * `0x00` : Nonaktifkan limit (Isi hingga 100%)
  * `uReservedIn` (UInt8Array): `[0, 0, 0, 0]`

---

## 5. Keyboard RGB 4-Zone (`AcerGamingFunction`)

AN515-57 memiliki 4 zona pencahayaan fisik (`KeyBoardArea = 5`, `PerKey = 0`).

### 5.1. Mengatur Warna Zona: `SetGamingLEDColor` & `SetGamingRgbKb`
* **Metode:** `AcerGamingFunction.SetGamingLEDColor(gmInput: UInt64, out gmOutput: UInt32)`
* **Format Payload Per Zona (UInt64):**
  ```text
  [Bits 31..24] : Zone Index (1 = Zone 1, 2 = Zone 2, 3 = Zone 3, 4 = Zone 4)
  [Bits 23..16] : Red Component (0x00 - 0xFF)
  [Bits 15..8]  : Green Component (0x00 - 0xFF)
  [Bits 7..0]   : Blue Component (0x00 - 0xFF)
  ```

### 5.2. Mengatur Efek Pencahayaan: `SetGamingLEDBehavior`
* **Metode:** `AcerGamingFunction.SetGamingLEDBehavior(gmInput: UInt64, out gmOutput: UInt32)`
* **Mode Efek:**
  * `0x00` : **Static** (Warna tetap menyala sesuai zona)
  * `0x01` : **Breathing** (Pulsing redup-terang perlahan)
  * `0x02` : **Wave** (Efek gelombang warna mengalir)
  * `0x08` : **Off** (Lampu keyboard mati total)

---

## 6. Status Overclocking AN515-57

* Berdasarkan inspeksi registry OEM dan spesifikasi hardware:
  * `HKLM\SOFTWARE\OEM\NitroSense\Overclock\CPU_OC_Support = 0`
  * `HKLM\SOFTWARE\OEM\NitroSense\Overclock\GPU_OC_Support = 0`
* **Keputusan Desain:** Fitur Overclocking tidak diimplementasikan pada NitroMan karena firmware AN515-57 mengunci multiplier prosesor (Non-HK/Locked).
