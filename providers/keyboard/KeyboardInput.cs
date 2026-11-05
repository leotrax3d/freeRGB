// Per-user lighting input only. No text conversion, files, network, key output or elevation.
using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

internal static class KeyboardInput
{
    private sealed class Event { internal int Scan; internal bool Extended, Pressed; internal long At, UnixMs; }
    private static readonly BlockingCollection<Event> Queue = new BlockingCollection<Event>(256);
    private static readonly Stopwatch Clock = Stopwatch.StartNew();
    private static readonly Hook Callback = OnKey;
    private static IntPtr hook;
    private static uint thread;
    private delegate IntPtr Hook(int code, IntPtr message, IntPtr data);
    [StructLayout(LayoutKind.Sequential)]
    private struct Message { internal IntPtr Window; internal uint Id; internal UIntPtr W; internal IntPtr L; internal uint Time; internal int X, Y; internal uint Private; }
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr SetWindowsHookEx(int id, Hook callback, IntPtr module, uint threadId);
    [DllImport("user32.dll")]
    private static extern bool UnhookWindowsHookEx(IntPtr value);
    [DllImport("user32.dll")]
    private static extern IntPtr CallNextHookEx(IntPtr value, int code, IntPtr message, IntPtr data);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr GetModuleHandle(string name);
    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")]
    private static extern bool PostThreadMessage(uint id, uint message, UIntPtr w, IntPtr l);
    [DllImport("user32.dll")]
    private static extern bool PeekMessage(out Message message, IntPtr window, uint min, uint max, uint remove);
    [DllImport("user32.dll")]
    private static extern int GetMessage(out Message message, IntPtr window, uint min, uint max);
    [DllImport("user32.dll")]
    private static extern bool TranslateMessage(ref Message message);
    [DllImport("user32.dll")]
    private static extern IntPtr DispatchMessage(ref Message message);

    private static IntPtr OnKey(int code, IntPtr message, IntPtr data)
    {
        // Never block or suppress keys. OS-injected macros are deliberately ignored.
        if (code == 0 && (message.ToInt64() == 0x100 || message.ToInt64() == 0x104 || message.ToInt64() == 0x101 || message.ToInt64() == 0x105))
        {
            int flags = Marshal.ReadInt32(data, 8);
            int scan = Marshal.ReadInt32(data, 4);
            bool extended = (flags & 1) != 0;
            if (Marshal.ReadInt32(data, 0) == 0x13) { scan = 0xe1; extended = false; } // Pause's E1 sequence, not Num Lock.
            if ((flags & 0x12) == 0 && scan > 0 && scan <= 255)
                Enqueue(new Event { Scan = scan, Extended = extended, Pressed = message.ToInt64() == 0x100 || message.ToInt64() == 0x104, At = Clock.ElapsedMilliseconds, UnixMs = DateTime.UtcNow.Ticks / TimeSpan.TicksPerMillisecond - 62135596800000L });
        }
        return CallNextHookEx(hook, code, message, data);
    }
    private static void Stop() { PostThreadMessage(thread, 0x12, UIntPtr.Zero, IntPtr.Zero); }
    private static void Enqueue(Event item) { try { Queue.TryAdd(item); } catch (InvalidOperationException) { } }
    private static string FormatEvent(Event item, long age)
    {
        // The low-level hook has no source identity. Never guess a device id.
        return "{\"version\":1,\"kind\":\"press\",\"scan_code\":" + item.Scan +
            ",\"extended\":" + (item.Extended ? "true" : "false") + ",\"age_ms\":" + age +
            ",\"observed_unix_ms\":" + item.UnixMs + ",\"pressed\":" + (item.Pressed ? "true" : "false") + ",\"device_id\":null}";
    }
    private static void WriteEvents()
    {
        try
        {
            Console.WriteLine("{\"version\":1,\"kind\":\"ready\"}");
            Console.Out.Flush();
            foreach (Event item in Queue.GetConsumingEnumerable())
            {
                if (item.Scan == 0) Console.WriteLine("{\"version\":1,\"kind\":\"heartbeat\"}");
                else
                {
                    long age = Clock.ElapsedMilliseconds - item.At;
                    if (age > 250) continue; // Never replay a backlog after a blocked reader.
                    Console.WriteLine(FormatEvent(item, age));
                }
                Console.Out.Flush();
            }
        }
        catch (IOException) { Stop(); }
    }
    private static int Main(string[] args)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        if (args.Length == 1 && args[0] == "--self-test") { Console.WriteLine("{\"version\":1,\"kind\":\"ready\"}"); return 0; }
        if (args.Length == 1 && args[0] == "--self-test-events")
        {
            Console.WriteLine(FormatEvent(new Event { Scan = 30, Pressed = true, UnixMs = 1000 }, 0));
            Console.WriteLine(FormatEvent(new Event { Scan = 30, Pressed = false, UnixMs = 1001 }, 0));
            return 0;
        }
        if (args.Length != 1 || args[0] != "--listen") return 2;
        thread = GetCurrentThreadId();
        Message message;
        PeekMessage(out message, IntPtr.Zero, 0, 0, 0); // Ensure EOF can always post WM_QUIT.
        hook = SetWindowsHookEx(13, Callback, GetModuleHandle(null), 0);
        if (hook == IntPtr.Zero) return 3;
        var writer = new Thread(WriteEvents) { IsBackground = true };
        var stop = new Thread(delegate() { try { Console.ReadLine(); } catch (IOException) { } finally { Stop(); } }) { IsBackground = true };
        try
        {
            writer.Start(); stop.Start();
            using (var heartbeat = new Timer(delegate { Enqueue(new Event()); }, null, 1000, 1000))
                while (GetMessage(out message, IntPtr.Zero, 0, 0) > 0) { TranslateMessage(ref message); DispatchMessage(ref message); }
        }
        finally { UnhookWindowsHookEx(hook); Queue.CompleteAdding(); writer.Join(1000); }
        return 0;
    }
}
