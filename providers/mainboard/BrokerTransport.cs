// Snapshot-only local transport. No paths, register access or setters cross this boundary.
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text;
using System.Threading.Tasks;
using Microsoft.Win32.SafeHandles;

internal static class BrokerTransport
{
    internal const string PipeName = "freergb-mainboard-v1";
    internal const int Limit = 262144;
    // GENERIC_WRITE also grants FILE_CREATE_PIPE_INSTANCE. Deliberately omit it.
    internal const uint ClientRights = 0x00100083;
    internal static readonly string DirectoryPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), "freeRGB", "MainboardBroker");
    internal static readonly string ServicePath = Path.Combine(DirectoryPath, "freergb-mainboard-service.exe");

    [StructLayout(LayoutKind.Sequential)]
    private struct SecurityAttributes { internal int Length; internal IntPtr Descriptor; internal int Inherit; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafePipeHandle CreateNamedPipe(string name, uint access, uint mode,
        uint instances, uint output, uint input, uint timeout, ref SecurityAttributes security);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafePipeHandle CreateFile(string name, uint access, uint share, IntPtr security,
        uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetNamedPipeServerProcessId(SafePipeHandle pipe, out uint process);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(uint access, bool inherit, uint process);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool QueryFullProcessImageName(IntPtr process, uint flags, StringBuilder path, ref uint size);
    [DllImport("kernel32.dll")]
    private static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool CancelIoEx(SafePipeHandle handle, IntPtr overlapped);

    private static void AwaitIo(Task operation, Stream stream, int timeout)
    {
        if (timeout > 0 && operation.Wait(timeout)) return;
        var pipe = stream as PipeStream;
        if (pipe != null)
        {
            CancelIoEx(pipe.SafePipeHandle, IntPtr.Zero);
            bool completed;
            try { completed = operation.Wait(1000); }
            catch (AggregateException) { completed = true; }
            if (!completed) throw new InvalidOperationException("Pipe I/O cancellation failed; stop the broker.");
        }
        throw new TimeoutException("Broker I/O timed out.");
    }

    internal static void Write(Stream stream, byte[] bytes)
    {
        AwaitIo(stream.WriteAsync(bytes, 0, bytes.Length), stream, 2000);
    }

    internal static bool ReadAcknowledgment(Stream stream)
    {
        byte[] ack = new byte[1];
        var read = stream.ReadAsync(ack, 0, 1);
        AwaitIo(read, stream, 2000);
        return read.Result == 1 && ack[0] == 6;
    }

    internal static NamedPipeServerStream CreateServer(string name, SecurityIdentifier user)
    {
        var acl = new PipeSecurity();
        acl.SetAccessRuleProtection(true, false);
        acl.AddAccessRule(new PipeAccessRule(new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null), PipeAccessRights.FullControl, AccessControlType.Allow));
        acl.AddAccessRule(new PipeAccessRule(new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null), PipeAccessRights.FullControl, AccessControlType.Allow));
        acl.AddAccessRule(new PipeAccessRule(user, (PipeAccessRights)ClientRights, AccessControlType.Allow));
        byte[] bytes = acl.GetSecurityDescriptorBinaryForm();
        IntPtr descriptor = Marshal.AllocHGlobal(bytes.Length);
        try
        {
            Marshal.Copy(bytes, 0, descriptor, bytes.Length);
            var attributes = new SecurityAttributes { Length = Marshal.SizeOf(typeof(SecurityAttributes)), Descriptor = descriptor };
            // Duplex, overlapped, FIRST_PIPE_INSTANCE; byte mode, REJECT_REMOTE_CLIENTS.
            SafePipeHandle handle = CreateNamedPipe(@"\\.\pipe\" + name, 0x40080003, 8, 1, Limit, 32, 0, ref attributes);
            if (handle.IsInvalid) { int error = Marshal.GetLastWin32Error(); handle.Dispose(); throw new Win32Exception(error); }
            try { return new NamedPipeServerStream(PipeDirection.InOut, true, false, handle); }
            catch { handle.Dispose(); throw; }
        }
        finally { Marshal.FreeHGlobal(descriptor); }
    }

    internal static NamedPipeClientStream Connect(string name)
    {
        // Overlapped + SECURITY_SQOS_PRESENT / SECURITY_IDENTIFICATION: even a
        // rejected server must not obtain an impersonation-capable client token.
        SafePipeHandle handle = CreateFile(@"\\.\pipe\" + name, ClientRights, 0, IntPtr.Zero, 3, 0x40110000, IntPtr.Zero);
        if (handle.IsInvalid) { int error = Marshal.GetLastWin32Error(); handle.Dispose(); throw new Win32Exception(error); }
        try { return new NamedPipeClientStream(PipeDirection.InOut, true, true, handle); }
        catch { handle.Dispose(); throw; }
    }

    internal static bool IsExpectedServer(NamedPipeClientStream pipe, string expectedPath)
    {
        uint pid;
        if (!GetNamedPipeServerProcessId(pipe.SafePipeHandle, out pid)) return false;
        IntPtr process = OpenProcess(0x1000, false, pid); // QUERY_LIMITED_INFORMATION; no VM read rights.
        if (process == IntPtr.Zero) return false;
        try
        {
            uint size = 32768;
            var path = new StringBuilder((int)size);
            return QueryFullProcessImageName(process, 0, path, ref size) &&
                String.Equals(Path.GetFullPath(path.ToString()), Path.GetFullPath(expectedPath), StringComparison.OrdinalIgnoreCase);
        }
        catch (Exception error)
        {
            if (error is Win32Exception || error is InvalidOperationException || error is ArgumentException || error is OverflowException) return false;
            throw;
        }
        finally { CloseHandle(process); }
    }

    // Fixed-size command avoids unbounded ReadLine allocation, including hostile clients.
    internal static bool ReadSnapshotRequest(Stream stream)
    {
        byte[] expected = Encoding.ASCII.GetBytes("snapshot\n");
        var clock = Stopwatch.StartNew();
        for (int i = 0; i < expected.Length; i++)
        {
            byte[] value = new byte[1];
            var read = stream.ReadAsync(value, 0, 1);
            int remaining = 2000 - checked((int)clock.ElapsedMilliseconds);
            AwaitIo(read, stream, remaining);
            if (read.Result != 1 || value[0] != expected[i]) return false;
        }
        return true;
    }

    internal static string ReadReply(Stream stream, int timeout)
    {
        var clock = Stopwatch.StartNew();
        using (var bytes = new MemoryStream())
        {
            byte[] buffer = new byte[1024];
            while (bytes.Length < Limit)
            {
                int remaining = timeout - checked((int)clock.ElapsedMilliseconds);
                if (remaining <= 0) throw new TimeoutException("Broker reply timed out.");
                var read = stream.ReadAsync(buffer, 0, Math.Min(buffer.Length, Limit - (int)bytes.Length));
                AwaitIo(read, stream, remaining);
                int count = read.Result;
                if (count == 0) throw new EndOfStreamException();
                int newline = Array.IndexOf(buffer, (byte)10, 0, count);
                if (newline >= 0)
                {
                    if (newline != count - 1) throw new InvalidDataException("Trailing broker data.");
                    bytes.Write(buffer, 0, newline);
                    return new UTF8Encoding(false, true).GetString(bytes.ToArray());
                }
                bytes.Write(buffer, 0, count);
            }
        }
        throw new InvalidDataException("Broker reply exceeds its size limit.");
    }

    internal static string Snapshot()
    {
        using (NamedPipeClientStream pipe = Connect(PipeName))
        {
            if (!IsExpectedServer(pipe, ServicePath)) throw new InvalidDataException("Untrusted motherboard broker identity.");
            // Checking the image path alone is insufficient if that path is writable.
            BrokerService.CheckProtectedPath(ServicePath);
            string directory = DirectoryPath;
            string programFiles = Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles);
            while (true)
            {
                BrokerService.CheckProtectedPath(directory);
                if (String.Equals(directory, programFiles, StringComparison.OrdinalIgnoreCase)) break;
                directory = Path.GetDirectoryName(directory);
                if (directory == null) throw new InvalidDataException("Invalid broker path.");
            }
            byte[] request = Encoding.ASCII.GetBytes("snapshot\n");
            Write(pipe, request);
            string reply = ReadReply(pipe, 2000);
            // Server keeps its exclusive pipe handle. Confirm receipt before disconnect.
            Write(pipe, new byte[] { 6 });
            return reply;
        }
    }
}
