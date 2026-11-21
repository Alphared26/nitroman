# NITRO-MAN HARDWARE CONTROL CENTER
## DESIGN SYSTEM & UI/UX SPECIFICATION: ANTI-AI-SLOP & INDUSTRIAL GAMING AESTHETIC
**Dokumen Panduan Desain & Penyempurnaan Antarmuka Native Hardware Utility**
*Target Referensi: Perpaduan Presisi Teknis G-Helper (ASUS) & Identitas Agresif NitroSense (Acer)*
*Perangkat Sasaran: Acer Nitro 5 AN515-57 (Core i7-11800H / RTX 3050 Ti)*

---

## 1. DIAGNOSIS MASALAH: MENGAPA TAMPILAN TERLIHAT SEPERTI "AI SLOP"

Banyak antarmuka pengguna hasil generasi kecerdasan buatan terjebak dalam perangkap visual web modern (Web 3.0 / Crypto SaaS / Generic Tailwind Dashboard). Ciri-ciri tersebut membuat aplikasi pengendali perangkat keras terlihat palsu, murah, dan tidak meyakinkan bagi pengguna laptop gaming.

### Karakteristik "AI Slop" yang Diharamkan
1. **Pill-Button & Super-Rounded Corners**: Penggunaan radius bulat berlebihan (`rounded-full`, `rounded-2xl`) yang identik dengan aplikasi mobile atau dashboard finansial santai.
2. **Generic Purple/Cyan Glow yang Berlebihan**: Efek `box-shadow: 0 0 40px` dengan warna ungu neon tanpa struktur hierarki cahaya yang logis.
3. **Ruang Kosong (White/Empty Space) Terlalu Boros**: Padding 32px ke atas yang menyia-nyiakan area layar, membuat informasi telemetri penting tersembunyi atau harus di-scroll.
4. **Diagram Hiasan Tanpa Nilai Teknis**: Ilustrasi airflow generik atau siluet kartun yang tidak merepresentasikan komponen fisik motherboard maupun heatsink laptop yang sesungguhnya.
5. **Penggunaan Karakter Emoji**: Emoji kartun pada tombol atau label status yang langsung merusak wibawa sebuah utilitas sistem tingkat kernel (ACPI EC).

### Karakteristik "Hardware Native" (G-Helper & NitroSense) yang Diadopsi
1. **Utilitarian Density (Kepadatan Data Efisien)**: Menampilkan puluhan parameter telemetri (suhu inti, putaran kipas RPM, frekuensi GHz, batas daya Watt, voltase) secara padat, presisi, dan mudah dibaca dalam satu pandangan mata tanpa scroll.
2. **Industrial Skeuomorphism**: Permukaan gelap matte bertekstur carbon fiber anodized, sudut panel chamfered 45 derajat hasil potongan mesin CNC, garis ukir (etched grooves), dan micro-bezel logam.
3. **Mechanical Tactile Switch**: Toggle dan tombol yang terasa seperti instrumen militer atau kokpit pesawat tempur, dengan transisi tegas, indikator LED SMD permukaan, dan status biner yang jelas (ACTIVE / LOCKED / BYPASS).
4. **Teknis & Terverifikasi**: Menampilkan nama perangkat keras asli, firmware ID (BIOS V1.18, EC 1.20), register hex WMI, serta diagram X-Ray internal yang presisi sesuai arsitektur Acer Nitro AN515-57.

---

## 2. ATURAN DESAIN INTI (DESIGN FOUNDATIONS)

### 2.1 Skema Warna Industri (Industrial Color Matrix)
Warna antarmuka tidak boleh menggunakan gradien pastel acak. Semua warna harus merujuk pada standar pencahayaan instrumen mesin industri dan OEM Acer Nitro:

| Kode Token | Nilai Hex | Fungsi & Peruntukan |
|:---|:---|:---|
| `--bg-chassis` | `#07080A` | Latar belakang terdalam chassis (ultra-dark carbon) |
| `--bg-panel` | `#0D1015` | Panel komponen utama dengan tekstur gunmetal teroksidasi |
| `--bg-module` | `#141820` | Kotak modul kontrol, tombol aktif, dan track slider |
| `--border-bevel` | `#242A35` | Garis batas luar bertekstur bevel logam |
| `--border-accent` | `#3A4454` | Sorotan tepi atas (light catch edge) untuk efek kedalaman 3D |
| `--nitro-red` | `#E4002B` | Warna aksen resmi Acer Nitro: Fan Max, Turbo, Critical Peak, WASD |
| `--interceptor-cyan`| `#00D2FF` | Aksen sekunder avionik: Telemetri CPU, Conservation Mode, Arrow Keys |
| `--hardware-amber` | `#FFAA00` | Indikator peringatan termal, beban RAM, mode baterai portabel |
| `--sensor-green` | `#00E676` | Status normal: EC Running, AC Plugged, SSD SMART Healthy |
| `--tx-primary` | `#E9ECF1` | Teks data utama, font angka monospaced berbobot tebal |
| `--tx-secondary` | `#8A94A6` | Label satuan teknis (RPM, W, MHz, Celcius) |
| `--tx-dim` | `#4E5666` | Deskripsi fungsional, keterangan parameter non-aktif |

### 2.2 Sistem Sudut & Bevel Potongan CNC (Chamfer Engineering)
Tidak ada sudut bulat sembarangan (`border-radius: 20px`). Seluruh panel dan kontainer wajib menggunakan geometri potong sudut 45 derajat (chamfer) khas laptop Predator & Nitro:

```css
/* Chamfer Panel Presisi CNC */
.industrial-card {
  position: relative;
  background: linear-gradient(145deg, #12161E 0%, #0A0C10 100%);
  border: 1px solid #252C3A;
  clip-path: polygon(
    12px 0, 
    100% 0, 
    100% calc(100% - 12px), 
    calc(100% - 12px) 100%, 
    0 100%, 
    0 12px
  );
  box-shadow: 
    inset 0 1px 0 rgba(255, 255, 255, 0.08),
    inset 0 0 20px rgba(0, 0, 0, 0.6),
    0 8px 24px rgba(0, 0, 0, 0.7);
}
```

### 2.3 Tipografi & Monospace Precision
- **Display & Header**: Menggunakan tipografi resmi **Acer Foco** (Black / Bold / Regular) yang diekstrak langsung dari NitroSense asli (`font_acerfoco_blk.ttf`).
- **Data & Telemetri**: Seluruh angka pembacaan suhu, clock, RPM, dan register wajib memakai font monospace berfitur `font-variant-numeric: tabular-nums;` agar angka tidak bergoyang atau bergeser saat nilai ter-update secara berkala.
- **Label Satuan**: Selalu berukuran lebih kecil, berwarna `--tx-secondary`, diletakkan langsung setelah angka (misal: `72` dengan `°C` kecil, bukan terpisah di baris lain).

---

## 3. PERBAIKAN SPESIFIK TIAP ELEMEN (TRANSFORMASI DARI AI SLOP KE HARDWARE NATIVE)

### 3.1 Header & Status Cockpit
- **Masalah Saat Ini**: Terlalu mirip navbar website konvensional.
- **Rancangan Baru**:
  - Logo diposisikan di bilah logam bersudut trapesium di sisi kiri atas: `NITRO` dalam tipografi logam perak, `MAN` dalam warna merah darah bertekstur menyala.
  - Subtitle: `AN515-57 ACPI CONTROL HUB • BIOS V1.18`.
  - Tombol Navigasi Tab dirancang menyerupai selector switch militer: berbentuk kotak bersudut tumpul dengan garis LED horizontal di bagian bawah yang menyala saat aktif.
  - Indikator Status Kanan: Bukan badge lonjong, melainkan 3 modul mini telemetri:
    1. `EC HUB`: Lampu LED SMD kotak mini berkedip hijau lembut (`0x00410009 ACTIVE`).
    2. `RAM RESIDENT`: Konsumsi memori instan (misal: `14.2 MB`) membuktikan aplikasi ringan setara G-Helper.
    3. Tombol Minimize Tray bertakik taktis.

### 3.2 Dual Tachometers (CPU & GPU Fan Clocks)
- **Masalah Saat Ini**: Tachometer terlihat seperti ring SVG generik dari chart library.
- **Rancangan Baru (Mengikuti NitroSense / Supercar Cluster)**:
  - Mengintegrasikan bezel logam asli NitroSense (`images_200_img_oc_indicator.png`).
  - Menempatkan bilah turbin kipas metalik asli (`images_200_img_fan.png`) di pusat lingkaran dengan bayangan radial dan efek putaran dinamis sesuai RPM riil.
  - Skala busur speedometer dilengkapi garis-garis tick ukir tipis (tick marks setiap 100 RPM, tick utama bernomor: `1000`, `2000`, `3000`, `4000`, `5000`, `MAX`).
  - Tiga kotak pembacaan digital di bawah masing-masing tachometer disusun horizontal dengan partisi garis terukir:
    - Kotak 1: `SUHU INTI` (dengan warna transisi dari cyan ke merah jika melebihi 85°C).
    - Kotak 2: `PUTARAN KIPAS` (RPM aktual hasil pembacaan register EC).
    - Kotak 3: `CLOCK SPEED` (Frekuensi instan core CPU dalam GHz).

### 3.3 Live Avionics Oscilloscope (Pengganti Diagram Airflow yang Dihapus)
- **Masalah Saat Ini**: Area tengah setelah penghapusan airflow memerlukan elemen yang berbobot data teknis tinggi.
- **Rancangan Baru**:
  - Dua modul grafik gelombang sensor analog (*oscilloscope*):
    - Jalur atas: Gelombang termal & watt CPU (Cyan).
    - Jalur bawah: Gelombang termal & VRAM GPU (Red).
  - Background grafik berupa *tactical millimeter grid* dengan garis putus-putus tipis berjarak presisi 20 piksel.
  - Garis kurva memiliki bayangan neon tipis (*sharp stroke* 2px dengan drop-shadow terkontrol, bukan blur tebal).
  - Dilengkapi teks telemetri lateral di kanan atas grafik:
    `PEAK: 74°C • AVG: 58°C • TDP: 45W • TJMAX: 100°C`.
  - Animasi pergeseran data berjalan mulus setiap 1 detik menyerupai instrumen radar avionik.

### 3.4 4-Zone RGB Deck (Layout Keyboard Asli AN515-57)
- **Masalah Saat Ini**: Banyak layout keyboard buatan AI yang tidak proporsional dan tidak mencerminkan keyboard fisik laptop aslinya.
- **Rancangan Baru**:
  - Rekonstruksi presisi chiclet keyboard Acer Nitro AN515-57 skala 1:1:
    - 6 baris lengkap: Function row (F1-F12), Number row, QWERTY, Home row, Shift row, dan Bottom row.
    - Numpad terdedikasi di sisi kanan lengkap dengan tombol khusus **Nitro Key `[N]`** bercangkang merah tebal.
    - Tombol **WASD** memiliki border merah kontras tebal dan permukaan huruf putih mencolok.
    - Tombol panah arah (`Arrow Keys`) memiliki penempatan sedikit turun ke bawah (offset) persis dengan sasis asli Nitro 5.
  - Backlight Diffusion Physics: Pendaran lampu tidak boleh berupa balok gradien kaku, melainkan *keycap underglow* yang berpendar keluar dari sela-sela antar-tuts (keygap) dan menembus ukiran huruf laser font Acer Foco.
  - Warna 4 zona dapat diatur mandiri atau serentak melalui tombol `Terapkan ke Semua`.
  - Pilihan efek lampu: `Static`, `Breathing`, `Wave / Spectrum`, `Zoom`, `Shifting`, dan `Off`.

### 3.5 System Specs: X-Ray CAD Hardware Architecture (Pengganti Mockup Kartun)
- **Masalah Saat Ini**: Mockup laptop 3D generik sering tampak seperti gambar kartun mainan.
- **Rancangan Baru**:
  - Menampilkan visualisasi **X-Ray / Blueprint Isometrik** komponen pendingin dan motherboard laptop Acer Nitro AN515-57:
    - Pipa pendingin ganda tembaga murni (*Dual Copper Heatpipes*) berwarna perunggu metalik yang menghubungkan die prosesor dan kartu grafis ke heatsink fin belakang.
    - Dua kipas blower di sudut kiri (GPU) dan kanan (CPU) dengan bilah aerodinamis.
    - Slot memori SODIMM ganda (16 GB DDR4-3200) dan port M.2 PCIe Gen4 NVMe.
  - Pin Telemetri Holografik Mengapung (Floating HUD Callout Pins):
    - Pin melayang dengan garis proyeksi titik putus-putus ke komponen internal:
      1. `[CPU]` Intel Core i7-11800H (8 Core / 16 Thread Tiger Lake-H, TDP 45W)
      2. `[GPU]` NVIDIA GeForce RTX 3050 Ti Laptop GPU (4 GB GDDR6, TGP 60W)
      3. `[RAM]` 16 GB Dual-Channel DDR4 3200 MHz
      4. `[SSD]` Micron 2450 NVMe PCIe Gen4 x4 512 GB
      5. `[BAT]` 57.5 Wh 4-Cell High-Density Li-ion
  - Interaksi Klik Pin: Mengklik pin CPU/GPU/RAM/SSD/Baterai secara instan menyorot kartu diagnostik di bawahnya dengan denyut cahaya (*neon pulse focus*).

### 3.6 Sakelar Kontrol (Mechanical Toggles & Rocker Switches)
- Mengganti toggle switch iPhone/Android dengan **Heavy-Duty Rocker Switch** bergaya panel industri militer:
  - Rangka persegi bertekstur bevel gelap dengan sudut potong.
  - Tuas logam persegi yang bergeser ke kanan dengan lampu LED garis merah/cyan saat berstatus ON.
  - Saat di-klik, terdapat respon transisi snap 80ms yang responsif.
  - Digunakan untuk: `CoolBoost`, `Conservation Mode 80%`, `LCD Overdrive 3ms`, dan `Windows Key Lock`.

---

## 4. INTEGRASI FITUR G-HELPER & NITROSENSE

Perbandingan fungsi yang disatukan ke dalam NitroMan:

```
[NITROSENSE RESMI ACER]            [G-HELPER ASUS ROG]
- Tampilan agresif gaming           - Ringan (< 20 MB RAM)
- CoolBoost fan curve shift         - Tanpa background service bloat
- Thermal Profiles (Quiet/Bal/Perf) - Kontrol batas baterai instan
- 4-Zone RGB Lighting Keyboard      - Akses langsung kurva kipas kustom
          \                                /
           \                              /
            v                            v
      [NITROMAN HYBRID HARDWARE CONTROL CENTER]
      - Ringan, responsif, dan instan (dibangun dengan Rust + Tauri)
      - UI bernuansa kokpit mesin tempur tanpa bloatware
      - Kontrol langsung register ACPI EC & WMI firmware laptop
      - Bekerja 100% offline tanpa dependensi cloud
```

### Fitur Eksklusif yang Disediakan
1. **Fan Mode Quick Matrix**:
   - `AUTO`: Mengembalikan kontrol kecepatan ke tabel termal BIOS firmware.
   - `MAX`: Mengunci putaran kedua kipas pada kecepatan tertinggi (hingga 5800 RPM).
   - `CUSTOM`: Mengaktifkan slider manual persentase atau kurva suhu responsif.
2. **CoolBoost Overdrive**: Menaikkan batas kurva RPM kipas ketika suhu inti CPU/GPU mendekati ambang throttling (efektif untuk sesi gaming intensif).
3. **Conservation Mode (Battery 80%)**: Berkomunikasi langsung dengan `ACCUtilities.dll` WMI driver untuk memutus arus pengisian pada kapasitas 80%, melindungi sel baterai dari degradasi panas.
4. **TuneUp & RAM Flush**: Mengosongkan cache memori standby Windows dan menjalankan perintah TRIM pada NVMe SSD dalam satu klik.
5. **DTS:X TrueHarmony Soundscapes**: Pengalih profil equalizer cepat untuk game FPS (`Shooter`), `RPG`, `Strategy`, `Movie`, dan `Music`.

---

## 5. STRUKTUR ARSITEKTUR KODE TAMPILAN

```
nitroman/
├── design/
│   ├── preview.html              <- Berkas uji antarmuka mandiri (dapat dibuka di browser)
│   └── assets/                   <- Link ke aset grafis resmi
├── nitroman-gui/
│   └── ui/
│       ├── index.html            <- Kode tampilan produksi aplikasi Tauri
│       └── assets/
│           ├── font_acerfoco_bd.ttf
│           ├── font_acerfoco_blk.ttf
│           ├── font_acerfoco_rg.ttf
│           ├── images_200_img_oc_indicator.png
│           ├── images_200_img_fan.png
│           └── acc/ (ikon Acer Care Center)
└── docs/
    └── design.md                 <- Spesifikasi arsitektur antarmuka ini
```

---

## 6. CHECKLIST PENILAIAN "ANTI-AI-SLOP"

Sebelum kode tampilan di-compile ke binary rilis, verifikasi setiap butir berikut:

- [x] **Zero Emojis**: Tidak ada satupun emoji di seluruh teks, label, console log, maupun alert dialog.
- [x] **Branding Konsisten**: Header bertuliskan `NITROMAN` dengan sub-header `ACER NITRO AN515-57`.
- [x] **Penghapusan Airflow**: Tidak ada diagram airflow dummy; digantikan sepenuhnya oleh oscilloscope telemetri live.
- [x] **Akurasi Keyboard AN515-57**: Layout fisik chiclet lengkap dengan tuts WASD merah, panah cyan, dan tombol khusus Nitro `[N]`.
- [x] **X-Ray / Blueprint System Specs**: Tampilan laptop isometrik dengan pin penunjuk modul komponen internal (CPU, GPU, RAM, SSD, Baterai).
- [x] **Tipografi Resmi OEM**: Font Acer Foco termuat secara lokal tanpa dependensi unduhan Google Fonts eksternal.
- [x] **Ketegasan Bevel**: Menggunakan chamfer 45 derajat hasil potongan CNC tajam, bukan rounded border bubble.
- [x] **Kinerja Ringan**: Halaman termuat dalam waktu di bawah 100 milidetik dan konsumsi RAM GUI di bawah 25 MB.

---
*Dokumen ini merupakan acuan resmi pengembangan visual NitroMan untuk menghasilkan aplikasi pengendali laptop gaming yang intimidatif, profesional, dan setara utilitas OEM kelas atas.*
