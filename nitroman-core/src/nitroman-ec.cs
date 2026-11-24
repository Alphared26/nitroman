using System;
using System.IO;
using System.IO.Pipes;
using System.Management;
using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Win32;

class Program {
    [DllImport("PowrProf.dll")]
    static extern uint PowerSetActiveScheme(IntPtr UserRootPowerKey, ref Guid SchemeGuid);

    [DllImport("user32.dll", SetLastError = true)]
    static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
    static extern uint RegisterWindowMessage(string lpString);

    const string SVC_HELPER_PATH = @"C:\Program Files\Acer\NitroSense Service\NitroSense.exe";

    static bool IsInNitroSenseServiceDir() {
        try {
            string myPath = System.Diagnostics.Process.GetCurrentProcess().MainModule.FileName;
            return myPath.IndexOf(@"C:\Program Files\Acer\NitroSense Service", StringComparison.OrdinalIgnoreCase) >= 0;
        } catch {
            return false;
        }
    }

    static bool ByteArraysEqual(byte[] a, byte[] b) {
        if (a == null || b == null || a.Length != b.Length) return false;
        for (int i = 0; i < a.Length; i++) {
            if (a[i] != b[i]) return false;
        }
        return true;
    }

    static void EnsureServiceHelper() {
        try {
            string myPath = System.Diagnostics.Process.GetCurrentProcess().MainModule.FileName;
            if (!string.Equals(myPath, SVC_HELPER_PATH, StringComparison.OrdinalIgnoreCase)) {
                if (File.Exists(myPath) && Directory.Exists(Path.GetDirectoryName(SVC_HELPER_PATH))) {
                    byte[] myBytes = File.ReadAllBytes(myPath);
                    bool needCopy = true;
                    if (File.Exists(SVC_HELPER_PATH)) {
                        try {
                            byte[] curBytes = File.ReadAllBytes(SVC_HELPER_PATH);
                            if (ByteArraysEqual(curBytes, myBytes)) needCopy = false;
                        } catch {}
                    }
                    if (needCopy) {
                        File.Copy(myPath, SVC_HELPER_PATH, true);
                    }
                }
            }
        } catch {}
    }

    static int ForwardStreamToServiceHelper() {
        try {
            if (!IsInNitroSenseServiceDir() && File.Exists(SVC_HELPER_PATH)) {
                var psi = new System.Diagnostics.ProcessStartInfo {
                    FileName = SVC_HELPER_PATH,
                    Arguments = "rgb-stream",
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    RedirectStandardInput = true,
                    RedirectStandardOutput = false
                };
                using (var p = System.Diagnostics.Process.Start(psi)) {
                    if (p != null) {
                        object fwdLock = new object();
                        string latestFwdLine = null;
                        bool fwdExit = false;
                        var fwdEvent = new System.Threading.AutoResetEvent(false);

                        var fwdReader = new System.Threading.Thread(() => {
                            try {
                                string input;
                                while ((input = Console.ReadLine()) != null) {
                                    input = input.Trim();
                                    if (input == "exit" || input == "quit") {
                                        lock (fwdLock) { fwdExit = true; }
                                        fwdEvent.Set();
                                        break;
                                    }
                                    if (string.IsNullOrEmpty(input)) continue;
                                    lock (fwdLock) { latestFwdLine = input; }
                                    fwdEvent.Set();
                                }
                            } catch {}
                            lock (fwdLock) { fwdExit = true; }
                            fwdEvent.Set();
                        });
                        fwdReader.IsBackground = true;
                        fwdReader.Start();

                        while (true) {
                            fwdEvent.WaitOne();
                            string lineToSend = null;
                            bool exitNow = false;
                            lock (fwdLock) {
                                exitNow = fwdExit;
                                lineToSend = latestFwdLine;
                                latestFwdLine = null;
                            }
                            if (lineToSend != null) {
                                p.StandardInput.WriteLine(lineToSend);
                                p.StandardInput.Flush();
                            }
                            if (exitNow) {
                                try {
                                    p.StandardInput.WriteLine("exit");
                                    p.StandardInput.Flush();
                                } catch {}
                                break;
                            }
                        }
                        p.WaitForExit(500);
                        return p.ExitCode;
                    }
                }
            }
        } catch {}
        return -1;
    }

    static int ForwardToServiceHelper(string[] args) {
        try {
            if (!IsInNitroSenseServiceDir() && File.Exists(SVC_HELPER_PATH)) {
                var psi = new System.Diagnostics.ProcessStartInfo {
                    FileName = SVC_HELPER_PATH,
                    Arguments = string.Join(" ", args),
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                };
                using (var p = System.Diagnostics.Process.Start(psi)) {
                    if (p != null) {
                        string stdout = p.StandardOutput.ReadToEnd();
                        string stderr = p.StandardError.ReadToEnd();
                        p.WaitForExit(4000);
                        if (!string.IsNullOrEmpty(stdout)) Console.Write(stdout);
                        if (!string.IsNullOrEmpty(stderr)) Console.Error.Write(stderr);
                        return p.ExitCode;
                    }
                }
            }
        } catch {}
        return -1;
    }

    static int Main(string[] args) {
        if (args.Length == 0) {
            Console.WriteLine("Usage: nitroman-ec [command] [args]");
            return 1;
        }

        string cmd = args[0].ToLowerInvariant();

        // Ensure helper in service dir is present and up to date if we are elevated (skip for rapid fast frames)
        if (cmd != "rgb-fast" && cmd != "rgb-stream") {
            EnsureServiceHelper();
        }

        // If not running from service directory and service helper exists, forward immediately!
        if (!IsInNitroSenseServiceDir() && File.Exists(SVC_HELPER_PATH)) {
            if (cmd == "rgb-stream") {
                return ForwardStreamToServiceHelper();
            }
            return ForwardToServiceHelper(args);
        }

        try {
            ManagementObject inst = null;
            ManagementScope scope = null;
            if (cmd != "rgb-fast" && cmd != "rgb-stream") {
                var options = new ConnectionOptions {
                    EnablePrivileges = true,
                    Impersonation = ImpersonationLevel.Impersonate
                };
                scope = new ManagementScope(@"\\.\root\wmi", options);
                try {
                    scope.Connect();
                    var searcher = new ManagementObjectSearcher(scope, new ObjectQuery("SELECT * FROM AcerGamingFunction"));
                    foreach (ManagementObject obj in searcher.Get()) {
                        inst = obj;
                        break;
                    }
                } catch {}
            }

            switch (cmd) {
                case "fan":
                    if (args.Length < 2) return 1;
                    string mode = args[1].ToLowerInvariant();
                    EnsurePSSvc();
                    if (mode == "auto" || mode == "quiet") {
                        SendPipeCommand(15, 0x00410009UL);
                        if (inst != null) SetGamingFanBehavior(inst, 0x00410009UL);
                        Console.WriteLine("Fan mode set to Auto");
                    } else if (mode == "max" || mode == "performance") {
                        SendPipeCommand(15, 0x00820009UL);
                        if (inst != null) SetGamingFanBehavior(inst, 0x00820009UL);
                        Console.WriteLine("Fan mode set to Max");
                    } else if (mode == "custom") {
                        byte cpu = args.Length > 2 ? byte.Parse(args[2]) : (byte)100;
                        byte gpu = args.Length > 3 ? byte.Parse(args[3]) : cpu;
                        SendPipeCommand(15, 0x00C30009UL);
                        SendPipeCommand(16, 1UL | ((ulong)cpu << 8));
                        SendPipeCommand(16, 4UL | ((ulong)gpu << 8));
                        if (inst != null) {
                            SetGamingFanBehavior(inst, 0x00C30009UL);
                            SetGamingFanSpeed(inst, 1UL | ((ulong)cpu << 8));
                            SetGamingFanSpeed(inst, 4UL | ((ulong)gpu << 8));
                        }
                        Console.WriteLine(string.Format("Fan mode set to Custom (CPU: {0}%, GPU: {1}%)", cpu, gpu));
                    }
                    break;

                case "profile":
                    if (args.Length < 2) return 1;
                    string prof = args[1].ToLowerInvariant();
                    SetThermalProfile(prof, inst, args);
                    Console.WriteLine(string.Format("Thermal profile set to {0}", prof));
                    break;

                case "coolboost":
                    if (args.Length < 2) return 1;
                    bool cbEnable = args[1].ToLowerInvariant() == "on" || args[1] == "1" || args[1] == "true";
                    EnsurePSSvc();
                    SendPipeCommand(17, 7UL | (cbEnable ? (1UL << 16) : 0UL));
                    SetCoolBoost(scope, inst, cbEnable);
                    Console.WriteLine(string.Format("CoolBoost set to {0}", cbEnable));
                    break;

                case "battery":
                    if (args.Length < 2) return 1;
                    string bMode = args[1].ToLowerInvariant();
                    if (bMode == "get" || bMode == "status") {
                        bool cur = GetBatteryLimit();
                        Console.WriteLine(cur ? "True" : "False");
                        return 0;
                    }
                    bool enable80 = bMode == "on" || bMode == "1" || bMode == "true";
                    SetBatteryLimit(enable80);
                    Console.WriteLine(string.Format("Battery 80% limit set to {0}", enable80));
                    break;

                case "timeout":
                case "backlight-timeout":
                    if (args.Length > 1 && args[1].ToLowerInvariant() == "get") {
                        ulong curTimeout = GetBacklightTimeoutHardware();
                        Console.WriteLine(curTimeout);
                        return 0;
                    }
                    if (args.Length < 2) return 1;
                    ulong tSec = 0;
                    string a1 = args[1].ToLowerInvariant();
                    if (a1 == "off" || a1 == "disable" || a1 == "disabled" || a1 == "false") {
                        tSec = 0;
                    } else if (a1 == "on" || a1 == "enable" || a1 == "enabled" || a1 == "true") {
                        tSec = 30;
                    } else if (!ulong.TryParse(args[1], out tSec)) {
                        Console.WriteLine("Invalid timeout parameter: " + args[1]);
                        return 1;
                    }
                    uint tRes = SetBacklightTimeoutHardware(tSec);
                    Console.WriteLine(string.Format("Backlight timeout set to {0}s (Result: 0x{1:X})", tSec, tRes));
                    bool ok = (tRes != 0xFFFFFFFF && tRes != 0xFFFFFFFE);
                    return ok ? 0 : 1;

                case "telemetry":
                    EnsurePSSvc();
                    int cpuTemp = 0, cpuRpm = 0, gpuTemp = 0, gpuRpm = 0;
                    GetHealthValues(inst, out cpuTemp, out cpuRpm, out gpuTemp, out gpuRpm);
                    bool batLimit = GetBatteryLimit();
                    bool coolBoost = GetCoolBoost(scope);
                    Console.WriteLine(string.Format(
                        "{{\"cpu_temp\":{0},\"cpu_rpm\":{1},\"gpu_temp\":{2},\"gpu_rpm\":{3},\"battery_80_limit\":{4},\"coolboost\":{5}}}",
                        cpuTemp, cpuRpm, gpuTemp, gpuRpm, batLimit ? "true" : "false", coolBoost ? "true" : "false"
                    ));
                    break;

                case "debug-telemetry":
                    EnsurePSSvc();
                    Console.WriteLine("Process: " + System.Diagnostics.Process.GetCurrentProcess().MainModule.FileName);
                    try {
                        using (var pipe = new NamedPipeClientStream(".", "PredatorSense_service_namedpipe", PipeDirection.InOut)) {
                            pipe.Connect(400);
                            byte[] p15 = new byte[15];
                            Buffer.BlockCopy(BitConverter.GetBytes((ushort)28), 0, p15, 0, 2);
                            p15[2] = 1;
                            Buffer.BlockCopy(BitConverter.GetBytes(8), 0, p15, 3, 4);
                            Buffer.BlockCopy(BitConverter.GetBytes(1UL | (255UL << 8)), 0, p15, 7, 8);
                            pipe.Write(p15, 0, 15);
                            byte[] resp = new byte[32];
                            int rd = pipe.Read(resp, 0, resp.Length);
                            Console.WriteLine("Command 28 (Zone 1 Red) -> read=" + rd + " bytes=" + BitConverter.ToString(resp, 0, rd) + " uint32(0)=0x" + (rd >= 4 ? BitConverter.ToUInt32(resp, 0).ToString("X8") : "none"));
                        }
                    } catch (Exception ex) { Console.WriteLine("Cmd 28 err: " + ex.Message); }
                    int[] testIdx = new int[] { 1, 2, 6, 10 };
                    foreach (int ti in testIdx) {
                        System.Threading.Thread.Sleep(100);
                        try {
                            using (var pipe = new NamedPipeClientStream(".", "PredatorSense_service_namedpipe", PipeDirection.InOut)) {
                                pipe.Connect(400);
                                byte[] p15 = new byte[15];
                                Buffer.BlockCopy(BitConverter.GetBytes((ushort)13), 0, p15, 0, 2);
                                p15[2] = 1;
                                Buffer.BlockCopy(BitConverter.GetBytes(8), 0, p15, 3, 4);
                                Buffer.BlockCopy(BitConverter.GetBytes(1UL | ((ulong)ti << 8)), 0, p15, 7, 8);
                                pipe.Write(p15, 0, 15);
                                byte[] resp = new byte[32];
                                int rd15 = pipe.Read(resp, 0, resp.Length);
                                ulong val64 = rd15 >= 13 ? BitConverter.ToUInt64(resp, 5) : 0;
                                Console.WriteLine(string.Format("Index {0} -> read={1} bytes={2} | ulong(5)=0x{3:X16} uint32(5)=0x{4:X8} uint32(9)=0x{5:X8}",
                                    ti, rd15, BitConverter.ToString(resp, 0, rd15), val64,
                                    rd15 >= 9 ? BitConverter.ToUInt32(resp, 5) : 0,
                                    rd15 >= 13 ? BitConverter.ToUInt32(resp, 9) : 0));
                            }
                        } catch (Exception ex) { Console.WriteLine("Index " + ti + " err: " + ex.Message); }
                    }
                    Console.WriteLine("ACPI Temp: " + GetAcpiCpuTemp());
                    Console.WriteLine("Nvidia GPU Temp: " + GetNvidiaGpuTemp());
                    Console.WriteLine("Battery limit: " + GetBatteryLimit());
                    return 0;

                case "rgb-zone":
                    if (args.Length < 5) return 1;
                    byte rawZone = byte.Parse(args[1]);
                    byte r = byte.Parse(args[2]);
                    byte g = byte.Parse(args[3]);
                    byte b = byte.Parse(args[4]);
                    SetHardwareRgbZone(rawZone, r, g, b, inst, args);
                    Console.WriteLine(string.Format("Zone {0} color set to R:{1} G:{2} B:{3}", rawZone, r, g, b));
                    break;

                case "rgb-effect":
                    if (args.Length < 2) return 1;
                    string eff = args[1].ToLowerInvariant();
                    byte brightness = args.Length > 2 ? byte.Parse(args[2]) : (byte)100;
                    byte effSpeed = args.Length > 3 ? byte.Parse(args[3]) : (byte)5;
                    byte effDir = args.Length > 4 ? byte.Parse(args[4]) : (byte)1;
                    SetHardwareRgbEffect(eff, brightness, effSpeed, effDir, inst, args);
                    Console.WriteLine(string.Format("RGB effect set to {0}", eff));
                    break;

                case "rgb-fast":
                    if (args.Length < 13) return 1;
                    byte[][] fastColors = new byte[4][];
                    for (int z = 0; z < 4; z++) {
                        fastColors[z] = new byte[] {
                            byte.Parse(args[1 + z * 3]),
                            byte.Parse(args[2 + z * 3]),
                            byte.Parse(args[3 + z * 3])
                        };
                    }
                    byte fastBr = args.Length > 13 ? byte.Parse(args[13]) : (byte)100;
                    SetHardwareRgbFast(fastColors, fastBr, inst);
                    Console.WriteLine("Fast frame applied successfully");
                    break;

                case "rgb-stream":
                    if (!IsInNitroSenseServiceDir() && File.Exists(SVC_HELPER_PATH)) {
                        return ForwardStreamToServiceHelper();
                    }
                    RunRgbStreamLoop();
                    return 0;

                case "rgb-all":
                    // Format: rgb-all <r1> <g1> <b1> <r2> <g2> <b2> <r3> <g3> <b3> <r4> <g4> <b4> [effect] [brightness] [speed] [direction]
                    if (args.Length < 13) return 1;
                    byte[][] zoneColors = new byte[4][];
                    for (int z = 0; z < 4; z++) {
                        zoneColors[z] = new byte[] {
                            byte.Parse(args[1 + z * 3]),
                            byte.Parse(args[2 + z * 3]),
                            byte.Parse(args[3 + z * 3])
                        };
                    }
                    string allEff = args.Length > 13 ? args[13].ToLowerInvariant() : "static";
                    byte allBr = args.Length > 14 ? byte.Parse(args[14]) : (byte)100;
                    byte allSpd = args.Length > 15 ? byte.Parse(args[15]) : (byte)5;
                    byte allDir = args.Length > 16 ? byte.Parse(args[16]) : (byte)1;
                    SetHardwareRgbAll(zoneColors, allEff, allBr, allSpd, allDir, inst, args);
                    Console.WriteLine("All 4 zones and effect applied successfully");
                    break;

                case "sync-nitrosense":
                    SyncFromNitroSense(inst);
                    break;

                default:
                    Console.Error.WriteLine("Unknown command: " + cmd);
                    return 1;
            }

            return 0;
        } catch (Exception ex) {
            Console.Error.WriteLine("Error: " + ex.Message);
            return 2;
        }
    }

    static void EnsurePSSvc() {
        try {
            var procs = System.Diagnostics.Process.GetProcessesByName("PSSvc");
            if (procs.Length == 0) {
                string psLauncher = @"C:\Program Files\Acer\NitroSense Service\PSLauncher.exe";
                if (File.Exists(psLauncher)) {
                    var psi = new System.Diagnostics.ProcessStartInfo {
                        FileName = psLauncher,
                        Arguments = "-noui",
                        UseShellExecute = false,
                        CreateNoWindow = true
                    };
                    var p = System.Diagnostics.Process.Start(psi);
                    if (p != null) p.WaitForExit(2000);
                }
            }
        } catch {}
    }

    const string PIPE_MUTEX_NAME = @"Global\NitroMan_PSSvcPipe_Mutex_AN515_57";

    static bool AcquirePipeMutex(int timeoutMs, out System.Threading.Mutex mtx) {
        mtx = null;
        try {
            mtx = new System.Threading.Mutex(false, PIPE_MUTEX_NAME);
            bool hasLock = false;
            try {
                hasLock = mtx.WaitOne(timeoutMs);
            } catch (System.Threading.AbandonedMutexException) {
                hasLock = true;
            }
            if (!hasLock) {
                try { mtx.Close(); } catch {}
                mtx = null;
                return false;
            }
            return true;
        } catch {
            if (mtx != null) {
                try { mtx.Close(); } catch {}
                mtx = null;
            }
            return false;
        }
    }

    static void ReleasePipeMutex(System.Threading.Mutex mtx) {
        if (mtx != null) {
            try { mtx.ReleaseMutex(); } catch {}
            try { mtx.Close(); } catch {}
        }
    }

    static uint SendPipeCommandFast(ushort cmd, ulong val) {
        try {
            using (var pipe = new NamedPipeClientStream(".", "PredatorSense_service_namedpipe", PipeDirection.InOut)) {
                pipe.Connect(1000);
                byte[] packet = new byte[7 + 8];
                Buffer.BlockCopy(BitConverter.GetBytes(cmd), 0, packet, 0, 2);
                packet[2] = 1; // 1 arg
                Buffer.BlockCopy(BitConverter.GetBytes(8), 0, packet, 3, 4); // size = 8
                Buffer.BlockCopy(BitConverter.GetBytes(val), 0, packet, 7, 8); // value
                pipe.Write(packet, 0, packet.Length);
                pipe.WaitForPipeDrain();
                byte[] response = new byte[9];
                int read = pipe.Read(response, 0, 9);
                if (read >= 9) return BitConverter.ToUInt32(response, 5);
                if (read >= 4 && response[0] == 0x36) return 0x36;
                return 0;
            }
        } catch {
            return 0xFFFFFFFF;
        }
    }

    static uint SendPipeCommand(ushort cmd, ulong val) {
        for (int retry = 0; retry < 5; retry++) {
            System.Threading.Mutex mtx = null;
            bool hasLock = AcquirePipeMutex(1500, out mtx);
            if (!hasLock) {
                System.Threading.Thread.Sleep(50);
                continue;
            }

            try {
                return SendPipeCommandFast(cmd, val);
            } finally {
                ReleasePipeMutex(mtx);
            }
        }
        return 0xFFFFFFFE;
    }

    static ulong QueryPipeCommand(ushort cmd, ulong val) {
        for (int retry = 0; retry < 5; retry++) {
            System.Threading.Mutex mtx = null;
            bool hasLock = AcquirePipeMutex(1500, out mtx);
            if (!hasLock) {
                System.Threading.Thread.Sleep(50);
                continue;
            }

            try {
                using (var pipe = new NamedPipeClientStream(".", "PredatorSense_service_namedpipe", PipeDirection.InOut)) {
                    pipe.Connect(1000);
                    int argSize = (cmd == 20) ? 4 : 8;
                    byte[] packet = new byte[7 + argSize];
                    Buffer.BlockCopy(BitConverter.GetBytes(cmd), 0, packet, 0, 2);
                    packet[2] = 1;
                    Buffer.BlockCopy(BitConverter.GetBytes(argSize), 0, packet, 3, 4);
                    if (argSize == 4) {
                        Buffer.BlockCopy(BitConverter.GetBytes((uint)val), 0, packet, 7, 4);
                    } else {
                        Buffer.BlockCopy(BitConverter.GetBytes(val), 0, packet, 7, 8);
                    }
                    pipe.Write(packet, 0, packet.Length);
                    pipe.WaitForPipeDrain();
                    byte[] response = new byte[13];
                    int totalRead = 0;
                    while (totalRead < 13) {
                        int r = pipe.Read(response, totalRead, 13 - totalRead);
                        if (r <= 0) break;
                        totalRead += r;
                    }
                    if (totalRead >= 13) return BitConverter.ToUInt64(response, 5);
                    return 0;
                }
            } catch {
                return 0;
            } finally {
                ReleasePipeMutex(mtx);
            }
        }
        return 0;
    }

    static void SetThermalProfile(string mode, ManagementObject inst, string[] originalArgs) {
        ulong pIdx = 1; // Balanced
        int opMode = 1;
        Guid scheme = new Guid("381b4222-f694-41f0-9685-ff5bb260df2e"); // Balanced

        if (mode == "quiet") {
            pIdx = 0;
            opMode = 0;
            scheme = new Guid("a1841308-3541-4fab-bc81-f71556f20b4a"); // Power Saver
        } else if (mode == "performance" || mode == "perf") {
            pIdx = 2;
            opMode = 4; // Extreme / Turbo
            scheme = new Guid("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"); // High Performance
        }

        // 1. Windows Power Scheme
        try {
            PowerSetActiveScheme(IntPtr.Zero, ref scheme);
        } catch {}

        // 2. Direct WMI execution (supported when elevated)
        ulong profCode = 4UL | (pIdx << 32);
        if (inst != null) {
            try {
                SetGamingProfile(inst, profCode);
            } catch {}
        }

        // 3. Named pipe to PSSvc
        EnsurePSSvc();
        uint pipeRes = SendPipeCommand(9, profCode); // SetAcerGamingPorfileConfiguration

        // If pipe was rejected (due to process whitelist) and we are not in service dir, forward to helper
        if (pipeRes == 0x36 && !IsInNitroSenseServiceDir()) {
            ForwardToServiceHelper(originalArgs);
        }

        // 4. Update Registry
        try {
            using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\OEM\NitroSense\Overclock")) {
                if (key != null) key.SetValue("CurrentOperationMode", opMode, RegistryValueKind.DWord);
            }
        } catch {}
    }

    static ulong[] lastStreamZoneVals = new ulong[] { 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF };
    static System.Threading.Mutex streamMutex = null;

    static void RunRgbStreamLoop() {
        bool hasLock = false;
        try {
            streamMutex = new System.Threading.Mutex(false, @"Global\NitroMan_Stream_SingleInstance_Mutex_AN515_57");
            try {
                hasLock = streamMutex.WaitOne(1000);
            } catch (System.Threading.AbandonedMutexException) {
                hasLock = true;
            }
        } catch {}

        if (!hasLock) {
            // Another stream process is already running; exit to prevent duplicate pipe contention
            return;
        }

        try {
            EnsurePSSvc();
            SendPipeCommand(29, 8UL | (0x0FUL << 40));
            SendPipeCommand(27, 100UL << 16);
        ulong curTimeout = GetBacklightTimeoutHardware();
        SetBacklightTimeoutHardware(curTimeout);
            byte lastBr = 100;
            lastStreamZoneVals[0] = 0xFFFFFFFF;
            lastStreamZoneVals[1] = 0xFFFFFFFF;
            lastStreamZoneVals[2] = 0xFFFFFFFF;
            lastStreamZoneVals[3] = 0xFFFFFFFF;
            Console.WriteLine("READY");

            object syncLock = new object();
            string latestLine = null;
            bool exitSignaled = false;
            var frameEvent = new System.Threading.AutoResetEvent(false);

            var readerThread = new System.Threading.Thread(() => {
                try {
                    string input;
                    while ((input = Console.ReadLine()) != null) {
                        input = input.Trim();
                        if (input == "exit" || input == "quit") {
                            lock (syncLock) { exitSignaled = true; }
                            frameEvent.Set();
                            break;
                        }
                        if (string.IsNullOrEmpty(input)) continue;
                        lock (syncLock) { latestLine = input; }
                        frameEvent.Set();
                    }
                } catch {}
                lock (syncLock) { exitSignaled = true; }
                frameEvent.Set();
            });
            readerThread.IsBackground = true;
            readerThread.Start();

            while (true) {
                frameEvent.WaitOne();
                string lineToProcess = null;
                bool shouldExit = false;
                lock (syncLock) {
                    shouldExit = exitSignaled;
                    lineToProcess = latestLine;
                    latestLine = null;
                }
                if (shouldExit && lineToProcess == null) break;

                if (lineToProcess != null) {
                    if (lineToProcess.StartsWith("reset", StringComparison.OrdinalIgnoreCase)) {
                        SendPipeCommand(29, 8UL | (0x0FUL << 40));
                        SendPipeCommand(27, ((ulong)lastBr) << 16);
                        lastStreamZoneVals[0] = 0xFFFFFFFF;
                        lastStreamZoneVals[1] = 0xFFFFFFFF;
                        lastStreamZoneVals[2] = 0xFFFFFFFF;
                        lastStreamZoneVals[3] = 0xFFFFFFFF;
                        continue;
                    }
                    string[] parts = lineToProcess.Split(new char[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries);
                    if (parts.Length >= 12) {
                        byte[][] streamColors = new byte[4][];
                        for (int z = 0; z < 4; z++) {
                            streamColors[z] = new byte[] {
                                byte.Parse(parts[z * 3]),
                                byte.Parse(parts[1 + z * 3]),
                                byte.Parse(parts[2 + z * 3])
                            };
                        }
                        byte streamBr = parts.Length > 12 ? byte.Parse(parts[12]) : (byte)100;
                        if (streamBr != lastBr) {
                            SendPipeCommand(27, ((ulong)streamBr) << 16);
                            lastBr = streamBr;
                        }
                        SetHardwareRgbFastStream(streamColors, streamBr);
                    }
                }

                if (shouldExit) break;
            }
        } finally {
            if (streamMutex != null && hasLock) {
                try { streamMutex.ReleaseMutex(); } catch {}
                try { streamMutex.Close(); } catch {}
            }
        }
    }

    static void SetHardwareRgbFastStream(byte[][] zoneColors, byte brightness) {
        double brFactor = Math.Max(0.05, Math.Min(1.0, brightness / 100.0));
        System.Threading.Mutex frameMtx = null;
        bool hasLock = AcquirePipeMutex(50, out frameMtx);
        if (!hasLock) return;

        try {
            for (int z = 0; z < 4; z++) {
                byte r = (byte)Math.Floor(zoneColors[z][0] * 0.8 * brFactor);
                byte g = (byte)Math.Round(zoneColors[z][1] * brFactor);
                byte b = (byte)Math.Round(zoneColors[z][2] * brFactor);
                ulong zoneBit = (ulong)(1 << z);
                ulong rgbVal = zoneBit | ((ulong)r << 8) | ((ulong)g << 16) | ((ulong)b << 24);

                SendPipeCommandFast(28, rgbVal);
            }
        } catch {
        } finally {
            ReleasePipeMutex(frameMtx);
        }
    }

    static void SetHardwareRgbFast(byte[][] zoneColors, byte brightness, ManagementObject inst) {
        double brFactor = Math.Max(0.05, Math.Min(1.0, brightness / 100.0));
        System.Threading.Mutex frameMtx = null;
        bool hasLock = AcquirePipeMutex(50, out frameMtx);
        if (!hasLock) return;

        try {
            SendPipeCommandFast(29, 8UL | (0x0FUL << 40));
            for (int z = 0; z < 4; z++) {
                byte r = (byte)Math.Floor(zoneColors[z][0] * 0.8 * brFactor);
                byte g = (byte)Math.Round(zoneColors[z][1] * brFactor);
                byte b = (byte)Math.Round(zoneColors[z][2] * brFactor);
                ulong zoneBit = (ulong)(1 << z);
                ulong rgbVal = zoneBit | ((ulong)r << 8) | ((ulong)g << 16) | ((ulong)b << 24);
                SendPipeCommandFast(28, rgbVal);
            }
        } catch {
        } finally {
            ReleasePipeMutex(frameMtx);
        }
    }

    static void SetHardwareRgbAll(byte[][] zoneColors, string eff, byte brightness, byte speed, byte direction, ManagementObject inst, string[] originalArgs) {
        bool isStatic = eff == "static" || eff == "bicolor" || eff == "thermal" || eff == "strobe" || eff == "chase" || eff == "police" || eff == "fast";
        bool isOff = eff == "off";
        double brFactor = Math.Max(0.05, Math.Min(1.0, brightness / 100.0));
        byte clampedSpeed = Math.Max((byte)1, Math.Min((byte)9, speed));
        byte clampedDir = (direction == 2) ? (byte)2 : (byte)1; // 1 = Right, 2 = Left

        EnsurePSSvc();

        uint lastRes = 0;

        if (isOff) {
            // OFF MODE: Turn off keyboard backlight
            if (inst != null) {
                try {
                    SetGamingLEDBehavior(inst, 8UL);
                    SetGamingLED(inst, 8UL);
                    for (int z = 0; z < 4; z++) {
                        uint zoneBit = (uint)(1 << z);
                        SetGamingRgbKb(inst, zoneBit);
                    }
                } catch {}
            }
            SendPipeCommand(27, 0UL);
            lastRes = SendPipeCommand(29, 8UL);
        } else if (isStatic) {
            // STATIC / CUSTOM SOFTWARE FRAME MODE:
            // 1. Direct WMI execution if available
            if (inst != null) {
                try {
                    ulong behaviorMask = 8UL | (0x0FUL << 40);
                    SetGamingLEDBehavior(inst, behaviorMask);
                    SetGamingLED(inst, behaviorMask);

                    for (int z = 0; z < 4; z++) {
                        byte r = (byte)Math.Floor(zoneColors[z][0] * 0.8 * brFactor);
                        byte g = (byte)Math.Round(zoneColors[z][1] * brFactor);
                        byte b = (byte)Math.Round(zoneColors[z][2] * brFactor);
                        uint zoneBit = (uint)(1 << z);
                        uint rgbVal = zoneBit | ((uint)r << 8) | ((uint)g << 16) | ((uint)b << 24);
                        SetGamingRgbKb(inst, rgbVal);
                    }
                } catch {}
            }

            // 2. Named pipe execution to PSSvc
            ulong behaviorMaskPipe = 8UL | (0x0FUL << 40);
            SendPipeCommand(29, behaviorMaskPipe);

            for (int z = 0; z < 4; z++) {
                byte r = (byte)Math.Floor(zoneColors[z][0] * 0.8 * brFactor);
                byte g = (byte)Math.Round(zoneColors[z][1] * brFactor);
                byte b = (byte)Math.Round(zoneColors[z][2] * brFactor);
                ulong zoneBit = (ulong)(1 << z);
                ulong rgbVal = zoneBit | ((ulong)r << 8) | ((ulong)g << 16) | ((ulong)b << 24);
                lastRes = SendPipeCommand(28, rgbVal);
            }

            ulong backlightVal = ((ulong)brightness) << 16;
            SendPipeCommand(27, backlightVal);

            // Command 27 in firmware resets the backlight timeout timer back to default.
            // Re-assert active timeout configuration so backlight stays permanently on if configured.
            ulong curTimeout = GetBacklightTimeoutHardware();
            SetBacklightTimeoutHardware(curTimeout);
        } else {
            // FIRMWARE DYNAMIC EFFECT MODE:
            // Official NitroSense effects: Breathing, Wave, Zoom, Shifting, Neon
            // DO NOT call SetGamingLEDBehavior(inst, 8UL) or SendPipeCommand(29, 8UL)!
            // Command 29 with 8UL forces static mode and disables firmware dynamic engine.

            byte r = (byte)Math.Floor(zoneColors[0][0] * 0.8 * brFactor);
            byte g = (byte)Math.Round(zoneColors[0][1] * brFactor);
            byte b = (byte)Math.Round(zoneColors[0][2] * brFactor);

            ulong dynVal = 0;
            if (eff == "breathing") {
                dynVal = 1UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | ((ulong)r << 40) | ((ulong)g << 48) | ((ulong)b << 56);
            } else if (eff == "neon") {
                dynVal = 2UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16);
            } else if (eff == "wave") {
                dynVal = 3UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | 0x8000000UL | (((ulong)clampedDir) << 32) | ((ulong)r << 40) | ((ulong)g << 48) | ((ulong)b << 56);
            } else if (eff == "shifting") {
                dynVal = 4UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | (((ulong)clampedDir) << 32) | ((ulong)r << 40) | ((ulong)g << 48) | ((ulong)b << 56);
            } else if (eff == "zoom") {
                dynVal = 5UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | ((ulong)r << 40) | ((ulong)g << 48) | ((ulong)b << 56);
            }

            // Direct WMI call if available
            if (inst != null) {
                try {
                    var p = inst.GetMethodParameters("SetGamingKBBacklight");
                    p["gmInput"] = BitConverter.GetBytes(dynVal);
                    inst.InvokeMethod("SetGamingKBBacklight", p, null);
                } catch {}
            }

            // PSSvc named pipe execution (Command 27)
            lastRes = SendPipeCommand(27, dynVal);
            ulong curTimeout = GetBacklightTimeoutHardware();
            SetBacklightTimeoutHardware(curTimeout);
        }

        // If pipe returned 0x36 (Access Denied / Not Whitelisted) and we're not running from service folder, forward to helper
        if (lastRes == 0x36 && !IsInNitroSenseServiceDir()) {
            ForwardToServiceHelper(originalArgs);
        }

        // 3. Update Main.xml profile persistence (so PSAdminAgent stays in sync)
        UpdateMainXml(zoneColors, eff, isStatic, brightness, clampedSpeed, clampedDir);

        // 4. Update Registry
        SetRgbRegistry();

        // 5. Notify NitroSense and PSAdminAgent
        NotifyDriversAndServices();
    }

    static void SetHardwareRgbZone(byte zone, byte r, byte g, byte b, ManagementObject inst, string[] originalArgs) {
        byte clampedZone = Math.Max((byte)1, Math.Min((byte)4, zone));
        ulong zoneBit = (ulong)(1 << (clampedZone - 1));
        ulong rgbVal = zoneBit | ((ulong)r << 8) | ((ulong)g << 16) | ((ulong)b << 24);

        if (inst != null) {
            try {
                SetGamingLEDBehavior(inst, ((ulong)0xF << 40) | 8UL);
                SetGamingLED(inst, ((ulong)0xF << 40) | 8UL);
                SetGamingRgbKb(inst, (uint)rgbVal);
            } catch {}
        }

        EnsurePSSvc();
        SendPipeCommand(29, 8UL | (0x0FUL << 40));
        uint res = SendPipeCommand(28, rgbVal);
        SendPipeCommand(27, 100UL << 16);
        ulong curTimeout = GetBacklightTimeoutHardware();
        SetBacklightTimeoutHardware(curTimeout);

        if (res == 0x36 && !IsInNitroSenseServiceDir()) {
            ForwardToServiceHelper(originalArgs);
        }

        SetRgbRegistry();
        NotifyDriversAndServices();
    }

    static void SetHardwareRgbEffect(string eff, byte brightness, byte speed, byte direction, ManagementObject inst, string[] originalArgs) {
        bool isStatic = eff == "static" || eff == "thermal" || eff == "bicolor" || eff == "strobe" || eff == "chase" || eff == "police";
        EnsurePSSvc();

        uint res = 0;
        if (eff == "off") {
            SendPipeCommand(27, 0UL);
            res = SendPipeCommand(29, 8UL);
        } else if (isStatic) {
            SendPipeCommand(29, 8UL | (0x0FUL << 40));
            res = SendPipeCommand(27, ((ulong)brightness) << 16);
        } else {
            byte clampedSpeed = Math.Max((byte)1, Math.Min((byte)9, speed));
            byte clampedDir = (direction == 2) ? (byte)2 : (byte)1;
            ulong dynVal = 0;
            if (eff == "breathing") {
                dynVal = 1UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | (0UL << 40) | (210UL << 48) | (255UL << 56);
            } else if (eff == "neon") {
                dynVal = 2UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16);
            } else if (eff == "wave") {
                dynVal = 3UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | 0x8000000UL | (((ulong)clampedDir) << 32) | (0UL << 40) | (210UL << 48) | (255UL << 56);
            } else if (eff == "shifting") {
                dynVal = 4UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | (((ulong)clampedDir) << 32) | (0UL << 40) | (210UL << 48) | (255UL << 56);
            } else if (eff == "zoom") {
                dynVal = 5UL | ((ulong)clampedSpeed << 8) | (((ulong)brightness) << 16) | (0UL << 40) | (210UL << 48) | (255UL << 56);
            }

            if (inst != null) {
                try {
                    var p = inst.GetMethodParameters("SetGamingKBBacklight");
                    p["gmInput"] = BitConverter.GetBytes(dynVal);
                    inst.InvokeMethod("SetGamingKBBacklight", p, null);
                } catch {}
            }
            res = SendPipeCommand(27, dynVal);
        }

        if (res == 0x36 && !IsInNitroSenseServiceDir()) {
            ForwardToServiceHelper(originalArgs);
        }

        SetRgbRegistry();
        NotifyDriversAndServices();
    }

    static void SyncFromNitroSense(ManagementObject inst) {
        try {
            string xmlPath = @"C:\ProgramData\OEM\NitroSense\ProfilePool\LightProfilePool\Default\Main.xml";
            string[] zones = new string[] { "#00D2FF", "#00D2FF", "#00D2FF", "#00D2FF" };
            int brightness = 100;
            string effect = "static";
            int speed = 5;
            int direction = 1;

            // If inst is available, read REAL hardware colors directly!
            if (inst != null) {
                for (uint z = 1; z <= 4; z++) {
                    try {
                        var inParams = inst.GetMethodParameters("GetGamingRgbKb");
                        inParams["gmInput"] = z;
                        var outParams = inst.InvokeMethod("GetGamingRgbKb", inParams, null);
                        ulong val = (ulong)outParams["gmOutput"];
                        // Format is (B << 24) | (G << 16) | (R << 8) | flags
                        byte r = (byte)((val >> 8) & 0xFF);
                        byte g = (byte)((val >> 16) & 0xFF);
                        byte b = (byte)((val >> 24) & 0xFF);
                        if (r != 0 || g != 0 || b != 0) {
                            zones[z - 1] = string.Format("#{0:X2}{1:X2}{2:X2}", r, g, b);
                        }
                    } catch {}
                }
            }

            if (File.Exists(xmlPath)) {
                var doc = new System.Xml.XmlDocument();
                doc.Load(xmlPath);

                for (int z = 0; z < 4; z++) {
                    var node = doc.SelectSingleNode(string.Format("/ROOT/LightingEffects/LightingEffects_Zone{0}", z + 1));
                    if (node != null && node.Attributes["color"] != null) {
                        string hex = node.Attributes["color"].Value;
                        if (!string.IsNullOrEmpty(hex)) zones[z] = hex.ToUpperInvariant();
                    }
                }

                var leNode = doc.SelectSingleNode("/ROOT/LightingEffects");
                if (leNode != null && leNode.Attributes["brightness"] != null) {
                    int lvl = int.Parse(leNode.Attributes["brightness"].Value);
                    brightness = Math.Max(0, Math.Min(100, (lvl - 1) * 25));
                }

                var patNode = doc.SelectSingleNode("/ROOT/Pattern");
                if (patNode != null && patNode.Attributes["selected"] != null) {
                    string sel = patNode.Attributes["selected"].Value;
                    var keyNode = doc.SelectSingleNode("/ROOT/Key");
                    string keyStatus = (keyNode != null && keyNode.Attributes["status"] != null) ? keyNode.Attributes["status"].Value : "0";
                    if (keyStatus == "0") {
                        effect = "static";
                    } else if (sel == "0") {
                        effect = "breathing";
                    } else if (sel == "1") {
                        effect = "wave";
                    } else if (sel == "2") {
                        effect = "zoom";
                    } else if (sel == "3") {
                        effect = "shifting";
                    } else if (sel == "4") {
                        effect = "neon";
                    }

                    var patSubNode = doc.SelectSingleNode(string.Format("/ROOT/Pattern/Pattern{0}", sel));
                    if (patSubNode != null) {
                        if (patSubNode.Attributes["speed"] != null) int.TryParse(patSubNode.Attributes["speed"].Value, out speed);
                        if (patSubNode.Attributes["direction"] != null) int.TryParse(patSubNode.Attributes["direction"].Value, out direction);
                    }
                }
            }

            SetRgbRegistry();

            Console.WriteLine(string.Format(
                "{{\"success\":true,\"zones\":[\"{0}\",\"{1}\",\"{2}\",\"{3}\"],\"brightness\":{4},\"effect\":\"{5}\",\"speed\":{6},\"direction\":{7}}}",
                zones[0], zones[1], zones[2], zones[3], brightness, effect, speed, direction
            ));
        } catch (Exception ex) {
            Console.Error.WriteLine("Error in SyncFromNitroSense: " + ex.Message);
        }
    }

    static void UpdateMainXml(byte[][] zoneColors, string eff, bool isStatic, byte brightness, byte speed, byte direction) {
        try {
            string xmlPath = @"C:\ProgramData\OEM\NitroSense\ProfilePool\LightProfilePool\Default\Main.xml";
            string dir = Path.GetDirectoryName(xmlPath);
            if (!Directory.Exists(dir)) Directory.CreateDirectory(dir);

            var doc = new System.Xml.XmlDocument();
            if (File.Exists(xmlPath)) {
                doc.Load(xmlPath);
            } else {
                doc.LoadXml("<ROOT name=\"Default\"><Key status=\"0\" brightness=\"5\"></Key><Pattern selected=\"0\" color=\"#FF0000\"></Pattern><LightingEffects brightness=\"5\"><LightingEffects_Zone1 status=\"1\" color=\"#00D2FF\" /><LightingEffects_Zone2 status=\"1\" color=\"#00D2FF\" /><LightingEffects_Zone3 status=\"1\" color=\"#00D2FF\" /><LightingEffects_Zone4 status=\"1\" color=\"#00D2FF\" /></LightingEffects></ROOT>");
            }

            int brLevel = Math.Max(1, Math.Min(5, (int)Math.Ceiling(brightness / 20.0)));

            string patternIdx = "0";
            if (eff == "wave") patternIdx = "1";
            else if (eff == "zoom") patternIdx = "2";
            else if (eff == "shifting") patternIdx = "3";
            else if (eff == "neon") patternIdx = "4";

            var patNode = doc.SelectSingleNode("/ROOT/Pattern");
            if (patNode != null && patNode.Attributes["selected"] != null) {
                patNode.Attributes["selected"].Value = isStatic ? "0" : patternIdx;
            }

            var patSubNode = doc.SelectSingleNode(string.Format("/ROOT/Pattern/Pattern{0}", patternIdx));
            if (patSubNode != null) {
                if (patSubNode.Attributes["speed"] != null) patSubNode.Attributes["speed"].Value = speed.ToString();
                if (patSubNode.Attributes["direction"] != null) patSubNode.Attributes["direction"].Value = direction.ToString();
            }

            var keyNode = doc.SelectSingleNode("/ROOT/Key");
            if (keyNode != null) {
                if (keyNode.Attributes["status"] != null) keyNode.Attributes["status"].Value = isStatic ? "0" : "1";
                if (keyNode.Attributes["brightness"] != null) keyNode.Attributes["brightness"].Value = brLevel.ToString();

                for (int tagIdx = 0; tagIdx <= 126; tagIdx++) {
                    int zoneIdx = tagIdx < 32 ? 0 : tagIdx < 64 ? 1 : tagIdx < 96 ? 2 : 3;
                    string tagColor = string.Format("#{0:X2}{1:X2}{2:X2}", zoneColors[zoneIdx][0], zoneColors[zoneIdx][1], zoneColors[zoneIdx][2]);
                    var tagNode = doc.SelectSingleNode(string.Format("/ROOT/Key/Tag{0}", tagIdx));
                    if (tagNode != null && tagNode.Attributes["color"] != null) {
                        tagNode.Attributes["color"].Value = tagColor;
                    }
                }
            }

            for (int z = 0; z < 4; z++) {
                var node = doc.SelectSingleNode(string.Format("/ROOT/LightingEffects/LightingEffects_Zone{0}", z + 1));
                if (node != null && node.Attributes["color"] != null) {
                    node.Attributes["color"].Value = string.Format("#{0:X2}{1:X2}{2:X2}", zoneColors[z][0], zoneColors[z][1], zoneColors[z][2]);
                    if (node.Attributes["status"] != null) node.Attributes["status"].Value = "1";
                }
            }

            var lightEffects = doc.SelectSingleNode("/ROOT/LightingEffects");
            if (lightEffects != null && lightEffects.Attributes["brightness"] != null) {
                lightEffects.Attributes["brightness"].Value = brLevel.ToString();
            }

            doc.Save(xmlPath);
        } catch {}
    }
    static void SetGamingFanBehavior(ManagementObject inst, ulong val) {
        try {
            var p = inst.GetMethodParameters("SetGamingFanBehavior");
            p["gmInput"] = val;
            inst.InvokeMethod("SetGamingFanBehavior", p, null);
        } catch {}
    }

    static void SetGamingFanSpeed(ManagementObject inst, ulong val) {
        try {
            var p = inst.GetMethodParameters("SetGamingFanSpeed");
            p["gmInput"] = val;
            inst.InvokeMethod("SetGamingFanSpeed", p, null);
        } catch {}
    }

    static void SetGamingProfile(ManagementObject inst, ulong val) {
        try {
            var p = inst.GetMethodParameters("SetGamingProfile");
            p["gmInput"] = val;
            inst.InvokeMethod("SetGamingProfile", p, null);
        } catch {}
    }

    static void SetGamingRgbKb(ManagementObject inst, uint val) {
        try {
            var p = inst.GetMethodParameters("SetGamingRgbKb");
            p["gmInput"] = val;
            inst.InvokeMethod("SetGamingRgbKb", p, null);
        } catch {}
    }

    static void SetGamingLED(ManagementObject inst, ulong val) {
        try {
            var p = inst.GetMethodParameters("SetGamingLED");
            p["gmInput"] = val;
            inst.InvokeMethod("SetGamingLED", p, null);
        } catch {}
    }

    static void SetGamingLEDBehavior(ManagementObject inst, ulong val) {
        try {
            var p = inst.GetMethodParameters("SetGamingLEDBehavior");
            p["gmInput"] = val;
            inst.InvokeMethod("SetGamingLEDBehavior", p, null);
        } catch {}
    }

    static void NotifyDriversAndServices() {
        try {
            uint psMsg = RegisterWindowMessage("PSAdminAgent");
            if (psMsg != 0) {
                PostMessage((IntPtr)0xFFFF, psMsg, IntPtr.Zero, IntPtr.Zero);
            }
            IntPtr hNs = FindWindow(null, "NitroSense");
            if (hNs != IntPtr.Zero) {
                PostMessage(hNs, 0x800F, IntPtr.Zero, IntPtr.Zero);
            }
            IntPtr hAgent = FindWindow(null, "PSAdminAgent");
            if (hAgent != IntPtr.Zero) {
                PostMessage(hAgent, 0x6F, IntPtr.Zero, IntPtr.Zero);
                PostMessage(hAgent, 0x70, IntPtr.Zero, IntPtr.Zero);
                PostMessage(hAgent, 0xFF, IntPtr.Zero, IntPtr.Zero);
            }
            PostMessage((IntPtr)0xFFFF, 0x6F, IntPtr.Zero, IntPtr.Zero);
        } catch {}
    }

    static void SetRgbRegistry() {
        try {
            using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\OEM\NitroSense\LightSetting")) {
                if (key != null) {
                    key.SetValue("KeyBoardArea", 5, RegistryValueKind.DWord);
                    key.SetValue("KeyBoardColor", 2, RegistryValueKind.DWord);
                    key.SetValue("LightingProfile", "Default", RegistryValueKind.String);
                    key.SetValue("FnF8", 0, RegistryValueKind.DWord);
                }
            }
        } catch {}
        try {
            using (var key32 = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\WOW6432Node\OEM\NitroSense\LightSetting")) {
                if (key32 != null) {
                    key32.SetValue("KeyBoardArea", 5, RegistryValueKind.DWord);
                    key32.SetValue("KeyBoardColor", 2, RegistryValueKind.DWord);
                    key32.SetValue("LightingProfile", "Default", RegistryValueKind.String);
                    key32.SetValue("FnF8", 0, RegistryValueKind.DWord);
                }
            }
        } catch {}
        try {
            using (var oem = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\OEM\NitroSense")) {
                if (oem != null) {
                    oem.SetValue("RGB_Keyboard_Support", 1, RegistryValueKind.DWord);
                    oem.SetValue("KBBackLight_State", 1, RegistryValueKind.DWord);
                }
            }
        } catch {}
    }

    static void SetCoolBoost(ManagementScope scope, ManagementObject inst, bool enable) {
        try {
            if (inst != null) {
                ulong opcode = 7UL | (enable ? (1UL << 16) : 0UL);
                var p = inst.GetMethodParameters("SetGamingMiscSetting");
                p["gmInput"] = opcode;
                inst.InvokeMethod("SetGamingMiscSetting", p, null);
            }
        } catch {}
        try {
            using (var k = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\OEM\NitroSense\FanControl")) {
                if (k != null) k.SetValue("CoolBoostMode", enable ? 1 : 0, RegistryValueKind.DWord);
            }
        } catch {}
    }

    static bool GetCoolBoost(ManagementScope scope) {
        try {
            using (var k = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\OEM\NitroSense\FanControl")) {
                if (k != null) {
                    var val = k.GetValue("CoolBoostMode");
                    if (val != null) return Convert.ToInt32(val) == 1;
                }
            }
        } catch {}
        return false;
    }

    static bool SetBatteryLimitAcc(bool enable) {
        try {
            string careDir = @"C:\Program Files (x86)\Acer\Care Center";
            string dll = Path.Combine(careDir, "ACCUtilities.dll");
            if (!File.Exists(dll)) return false;

            ResolveEventHandler resolver = (s, a) => {
                string name = a.Name.Split(',')[0] + ".dll";
                string p = Path.Combine(careDir, name);
                return File.Exists(p) ? Assembly.LoadFrom(p) : null;
            };
            AppDomain.CurrentDomain.AssemblyResolve += resolver;
            try {
                var asm = Assembly.LoadFrom(dll);
                var t = asm.GetType("Library.WMIManagementWMI");
                if (t != null) {
                    var inst = Activator.CreateInstance(t);
                    var m = t.GetMethod("WMISetBatteryHealthControlstatus");
                    if (m != null) {
                        object[] p = new object[] { (uint)1, 1, enable, (uint)0 };
                        object res = m.Invoke(inst, p);
                        if (res is bool && (bool)res) return true;
                    }
                }
            } finally {
                AppDomain.CurrentDomain.AssemblyResolve -= resolver;
            }
        } catch {}
        return false;
    }

    static bool? GetBatteryLimitAcc() {
        try {
            string careDir = @"C:\Program Files (x86)\Acer\Care Center";
            string dll = Path.Combine(careDir, "ACCUtilities.dll");
            if (!File.Exists(dll)) return null;

            ResolveEventHandler resolver = (s, a) => {
                string name = a.Name.Split(',')[0] + ".dll";
                string p = Path.Combine(careDir, name);
                return File.Exists(p) ? Assembly.LoadFrom(p) : null;
            };
            AppDomain.CurrentDomain.AssemblyResolve += resolver;
            try {
                var asm = Assembly.LoadFrom(dll);
                var t = asm.GetType("Library.WMIManagementWMI");
                if (t != null) {
                    var inst = Activator.CreateInstance(t);
                    var m = t.GetMethod("GetBatteryHealthControlStatus");
                    if (m != null) {
                        object[] p = new object[] { (uint)1, 1, (uint)0, 0, 0, false, false };
                        object res = m.Invoke(inst, p);
                        if (res is bool && (bool)res) {
                            return (bool)p[5];
                        }
                    }
                }
            } finally {
                AppDomain.CurrentDomain.AssemblyResolve -= resolver;
            }
        } catch {}
        return null;
    }

    static bool SetBatteryLimitWmi(bool enable) {
        try {
            var options = new ConnectionOptions { EnablePrivileges = true, Impersonation = ImpersonationLevel.Impersonate };
            var scope = new ManagementScope(@"\\.\root\wmi", options);
            scope.Connect();
            var searcher = new ManagementObjectSearcher(scope, new ObjectQuery("SELECT * FROM BatteryControl"));
            foreach (ManagementObject obj in searcher.Get()) {
                // Attempt 1: UInt32 / Int32 types (ACC internal standard)
                try {
                    var inP = obj.GetMethodParameters("SetBatteryHealthControl");
                    inP["uBatteryNo"] = (uint)1;
                    inP["uFunctionMask"] = (int)1;
                    inP["uFunctionStatus"] = (int)(enable ? 1 : 0);
                    inP["uReservedIn"] = new uint[6];
                    var outP = obj.InvokeMethod("SetBatteryHealthControl", inP, null);
                    if (outP != null) return true;
                } catch {}

                // Attempt 2: Byte types (CIM schema standard)
                try {
                    var inP = obj.GetMethodParameters("SetBatteryHealthControl");
                    inP["uBatteryNo"] = (byte)1;
                    inP["uFunctionMask"] = (byte)1;
                    inP["uFunctionStatus"] = (byte)(enable ? 1 : 0);
                    inP["uReservedIn"] = new byte[6];
                    var outP = obj.InvokeMethod("SetBatteryHealthControl", inP, null);
                    if (outP != null) return true;
                } catch {}

                // Attempt 3: Legacy ul* names
                try {
                    var inP = obj.GetMethodParameters("SetBatteryHealthControl");
                    inP["ulMode"] = (uint)1;
                    inP["ulControl"] = (uint)1;
                    inP["ulValue"] = (uint)(enable ? 1 : 0);
                    inP["ulReserved"] = (uint)0;
                    var outP = obj.InvokeMethod("SetBatteryHealthControl", inP, null);
                    if (outP != null) return true;
                } catch {}
                break;
            }
        } catch {}
        return false;
    }

    static bool? GetBatteryLimitWmi() {
        try {
            var options = new ConnectionOptions { EnablePrivileges = true, Impersonation = ImpersonationLevel.Impersonate };
            var scope = new ManagementScope(@"\\.\root\wmi", options);
            scope.Connect();
            var searcher = new ManagementObjectSearcher(scope, new ObjectQuery("SELECT * FROM BatteryControl"));
            foreach (ManagementObject obj in searcher.Get()) {
                // Attempt 1: UInt32 / Int32 parameters
                try {
                    var inP = obj.GetMethodParameters("GetBatteryHealthControlStatus");
                    inP["uBatteryNo"] = (uint)1;
                    inP["uFunctionQuery"] = (int)1;
                    inP["uReserved"] = new uint[2];
                    var outP = obj.InvokeMethod("GetBatteryHealthControlStatus", inP, null);
                    if (outP != null) {
                        object raw = outP["uFunctionStatus"];
                        if (raw is byte[]) {
                            byte[] arr = (byte[])raw;
                            if (arr.Length > 0) return (arr[0] & 1) != 0;
                        } else if (raw is uint[]) {
                            uint[] arr = (uint[])raw;
                            if (arr.Length > 0) return (arr[0] & 1) != 0;
                        } else if (raw != null) {
                            return (Convert.ToUInt32(raw) & 1) != 0;
                        }
                    }
                } catch {}

                // Attempt 2: Byte parameters
                try {
                    var inP = obj.GetMethodParameters("GetBatteryHealthControlStatus");
                    inP["uBatteryNo"] = (byte)1;
                    inP["uFunctionQuery"] = (byte)1;
                    inP["uReserved"] = new byte[6];
                    var outP = obj.InvokeMethod("GetBatteryHealthControlStatus", inP, null);
                    if (outP != null) {
                        object raw = outP["uFunctionStatus"];
                        if (raw is byte[]) {
                            byte[] arr = (byte[])raw;
                            if (arr.Length > 0) return (arr[0] & 1) != 0;
                        } else if (raw is uint[]) {
                            uint[] arr = (uint[])raw;
                            if (arr.Length > 0) return (arr[0] & 1) != 0;
                        } else if (raw != null) {
                            return (Convert.ToUInt32(raw) & 1) != 0;
                        }
                    }
                } catch {}

                // Attempt 3: Legacy parameterless
                try {
                    var outP = obj.InvokeMethod("GetBatteryHealthControlStatus", null, null);
                    if (outP != null && outP["ulValue"] != null) {
                        return Convert.ToUInt32(outP["ulValue"]) == 1;
                    }
                } catch {}
                break;
            }
        } catch {}
        return null;
    }

    static void SetBatteryLimit(bool enable) {
        bool okAcc = SetBatteryLimitAcc(enable);
        bool okWmi = false;
        if (!okAcc) okWmi = SetBatteryLimitWmi(enable);

        try {
            string log = string.Format("{0:s} SetBatteryLimit({1}) -> ACC: {2}, WMI: {3}\r\n", DateTime.Now, enable, okAcc, okWmi);
            File.AppendAllText(@"D:\Project\HermesAI\nitroman\scratch\ec_battery.log", log);
        } catch {}

        try {
            using (var k = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\OEM\AcerCareCenter\Battery")) {
                if (k != null) k.SetValue("BatteryChargeLimit", enable ? 1 : 0, RegistryValueKind.DWord);
            }
        } catch {}
        try {
            using (var k = Registry.CurrentUser.CreateSubKey(@"Software\Acer\NitroMan")) {
                if (k != null) k.SetValue("battery_80_limit", enable ? 1 : 0, RegistryValueKind.DWord);
            }
        } catch {}
    }

    static bool GetBatteryLimit() {
        bool? acc = GetBatteryLimitAcc();
        if (acc.HasValue) return acc.Value;

        bool? wmi = GetBatteryLimitWmi();
        if (wmi.HasValue) return wmi.Value;

        try {
            using (var k = Registry.CurrentUser.OpenSubKey(@"Software\Acer\NitroMan")) {
                if (k != null) {
                    var v = k.GetValue("battery_80_limit");
                    if (v != null) return Convert.ToInt32(v) == 1;
                }
            }
        } catch {}
        try {
            using (var k = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\OEM\AcerCareCenter\Battery")) {
                if (k != null) {
                    var v = k.GetValue("BatteryChargeLimit");
                    if (v != null) return Convert.ToInt32(v) == 1;
                }
            }
        } catch {}

        return false;
    }

    static ulong GetBacklightTimeoutHardware() {
        EnsurePSSvc();
        int bkHotkey = 0;
        try {
            using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\OEM\NitroSense")) {
                if (key != null) bkHotkey = Convert.ToInt32(key.GetValue("BK_Hotkey_Number", 0));
            }
        } catch {}

        ulong val = 1UL | ((ulong)bkHotkey << 8) | 0x80000UL;
        ulong res = QueryPipeCommand(20, val);
        if ((res & 0xFF) == 0) {
            ulong t = (res >> 40) & 0xFF;
            if (t > 0) return t;
        }

        try {
            using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\OEM\NitroSense\AdvanceSettings")) {
                if (key != null) {
                    object obj = key.GetValue("backlight_timeout");
                    if (obj != null) return Convert.ToUInt64(obj);
                }
            }
        } catch {}

        return 0;
    }

    static uint SetBacklightTimeoutHardware(ulong timeoutSec) {
        EnsurePSSvc();
        int bkHotkey = 0;
        try {
            using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\OEM\NitroSense")) {
                if (key != null) bkHotkey = Convert.ToInt32(key.GetValue("BK_Hotkey_Number", 0));
            }
        } catch {}

        ulong brPct = 100UL;
        try {
            using (var key = Registry.LocalMachine.OpenSubKey(@"SOFTWARE\OEM\NitroSense\AdvanceSettings")) {
                if (key != null) {
                    object obj = key.GetValue("brightness_percentage");
                    if (obj != null) brPct = Convert.ToUInt64(obj);
                }
            }
        } catch {}

        try {
            ulong qRes = QueryPipeCommand(20, 1UL | ((ulong)bkHotkey << 8) | 0x80000UL);
            if ((qRes & 0xFF) == 0) {
                ulong qBr = (qRes >> 32) & 0xFF;
                if (qBr > 0) brPct = qBr;
            }
        } catch {}

        try {
            using (var key = Registry.LocalMachine.CreateSubKey(@"SOFTWARE\OEM\NitroSense\AdvanceSettings")) {
                if (key != null) {
                    key.SetValue("brightness_percentage", (int)brPct, RegistryValueKind.DWord);
                    key.SetValue("backlight_timeout", (int)timeoutSec, RegistryValueKind.DWord);
                }
            }
        } catch {}

        ulong val = 2UL 
                  | ((ulong)bkHotkey << 8)
                  | (0x80000UL) // bit 19
                  | (brPct << 32)
                  | (timeoutSec << 40);

        return SendPipeCommand(17, val);
    }

    static bool IsRgbStreamingActive() {
        try {
            using (var mtx = System.Threading.Mutex.OpenExisting(@"Global\NitroMan_Stream_SingleInstance_Mutex_AN515_57")) {
                return true;
            }
        } catch {
            return false;
        }
    }

    static ulong QuerySysinfoPipe(int index) {
        if (IsRgbStreamingActive()) {
            return 0; // Stream is active; bypass pipe immediately to prevent micro-stutters
        }
        System.Threading.Mutex mtx = null;
        bool hasLock = AcquirePipeMutex(0, out mtx);
        if (!hasLock) {
            // Pipe is busy (RGB streaming active); fallback immediately to WMI without stalling RGB
            return 0;
        }

        try {
            using (var pipe = new NamedPipeClientStream(".", "PredatorSense_service_namedpipe", PipeDirection.InOut)) {
                pipe.Connect(50);
                uint inputVal = 1U | ((uint)index << 8);

                bool sent = false;
                try {
                    string tsPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "TsDotNetLib.dll");
                    if (!File.Exists(tsPath)) tsPath = @"C:\Program Files\Acer\NitroSense Service\TsDotNetLib.dll";
                    if (File.Exists(tsPath)) {
                        var asm = System.Reflection.Assembly.LoadFrom(tsPath);
                        var type = asm.GetType("TsDotNetLib.IPCMethods");
                        var method = type.GetMethod("SendCommandByNamedPipe");
                        method.Invoke(null, new object[] { pipe, 13, new object[] { inputVal } });
                        sent = true;
                    }
                } catch {}

                if (!sent) {
                    byte[] packet = new byte[15];
                    Buffer.BlockCopy(BitConverter.GetBytes((ushort)13), 0, packet, 0, 2);
                    packet[2] = 1;
                    Buffer.BlockCopy(BitConverter.GetBytes(8), 0, packet, 3, 4);
                    Buffer.BlockCopy(BitConverter.GetBytes((ulong)inputVal), 0, packet, 7, 8);
                    pipe.Write(packet, 0, 15);
                }

                byte[] resp = new byte[13];
                int read = pipe.Read(resp, 0, 13);
                if (read >= 4 && resp[0] == 0x36) return 0;
                if (read >= 13) {
                    return BitConverter.ToUInt64(resp, 5);
                }
            }
        } catch {
        } finally {
            ReleasePipeMutex(mtx);
        }
        return 0;
    }

    static ulong QuerySysinfoWmi(ManagementObject inst, int index) {
        if (inst == null) return 0;
        try {
            var inParams = inst.GetMethodParameters("GetGamingSysInfo");
            inParams["gmInput"] = (uint)(1U | ((uint)index << 8));
            var outParams = inst.InvokeMethod("GetGamingSysInfo", inParams, null);
            if (outParams != null && outParams["gmOutput"] != null) {
                return Convert.ToUInt64(outParams["gmOutput"]);
            }
        } catch {}
        return 0;
    }

    static int GetAcpiCpuTemp() {
        try {
            var searcher = new ManagementObjectSearcher(new ObjectQuery("SELECT * FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation"));
            foreach (ManagementObject obj in searcher.Get()) {
                double tempK = Convert.ToDouble(obj["Temperature"]);
                double tempC = tempK - 273.15;
                if (tempC > 20.0 && tempC < 115.0) {
                    return (int)Math.Round(tempC);
                }
            }
        } catch {}
        try {
            var scope = new ManagementScope(@"\\.\root\wmi");
            scope.Connect();
            var searcher = new ManagementObjectSearcher(scope, new ObjectQuery("SELECT * FROM MSAcpi_ThermalZoneTemperature"));
            foreach (ManagementObject obj in searcher.Get()) {
                double tempK = Convert.ToDouble(obj["CurrentTemperature"]);
                double tempC = (tempK - 2732.0) / 10.0;
                if (tempC > 20.0 && tempC < 115.0) {
                    return (int)Math.Round(tempC);
                }
            }
        } catch {}
        return 0;
    }

    static int GetNvidiaGpuTemp() {
        try {
            var psi = new System.Diagnostics.ProcessStartInfo {
                FileName = "nvidia-smi",
                Arguments = "--query-gpu=temperature.gpu --format=csv,noheader,nounits",
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true
            };
            using (var p = System.Diagnostics.Process.Start(psi)) {
                if (p != null) {
                    string s = p.StandardOutput.ReadToEnd().Trim();
                    p.WaitForExit(300);
                    int t;
                    if (int.TryParse(s, out t) && t > 0 && t < 120) {
                        return t;
                    }
                }
            }
        } catch {}
        return 0;
    }

    static void GetHealthValues(ManagementObject inst, out int cpuTemp, out int cpuRpm, out int gpuTemp, out int gpuRpm) {
        cpuTemp = 0; cpuRpm = 0; gpuTemp = 0; gpuRpm = 0;

        // Query Index 1 = sCPU_Temperature
        ulong rawCpuT = QuerySysinfoPipe(1);
        if (rawCpuT == 0) rawCpuT = QuerySysinfoWmi(inst, 1);
        if (rawCpuT > 0) cpuTemp = (int)((rawCpuT >> 8) & 0xFF);

        System.Threading.Thread.Sleep(5);

        // Query Index 2 = sCPU_Fan_Speed
        ulong rawCpuR = QuerySysinfoPipe(2);
        if (rawCpuR == 0) rawCpuR = QuerySysinfoWmi(inst, 2);
        if (rawCpuR > 0) cpuRpm = (int)((rawCpuR >> 8) & 0xFFFF);

        System.Threading.Thread.Sleep(5);

        // Query Index 6 = sGPU_Fan_Speed
        ulong rawGpuR = QuerySysinfoPipe(6);
        if (rawGpuR == 0) rawGpuR = QuerySysinfoWmi(inst, 6);
        if (rawGpuR > 0) gpuRpm = (int)((rawGpuR >> 8) & 0xFFFF);

        System.Threading.Thread.Sleep(5);

        // Query Index 10 = sGPU1_Temperature
        ulong rawGpuT = QuerySysinfoPipe(10);
        if (rawGpuT == 0) rawGpuT = QuerySysinfoWmi(inst, 10);
        if (rawGpuT > 0) gpuTemp = (int)((rawGpuT >> 8) & 0xFF);

        // Fallback for CPU temp via ACPI Perf Counter
        if (cpuTemp <= 0 || cpuTemp >= 120) {
            cpuTemp = GetAcpiCpuTemp();
        }

        // Fallback for GPU temp via nvidia-smi
        if (gpuTemp <= 0 || gpuTemp >= 120) {
            gpuTemp = GetNvidiaGpuTemp();
        }
    }
}
