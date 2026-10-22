using System;
using System.IO;
using System.Text;

internal static class DeploymentTests
{
    private sealed class Registration : IBrokerRegistration
    {
        internal bool present, active, failStart, failStop;
        internal int starts;
        public bool Exists { get { return present; } }
        public bool Running { get { return active; } }
        public void Create() { present = true; }
        public void Remove() { present = false; }
        public void Stop() { if (failStop) throw new IOException("stop failed"); active = false; }
        public void Start() { starts++; if (failStart) { failStart = false; throw new IOException("new version failed"); } active = true; }
    }
    private static void Assert(bool condition, string message) { if (!condition) throw new Exception(message); }
    private static void Transactions(string parent)
    {
        foreach (string scenario in new[] { "new", "update", "rollback-running", "rollback-stopped", "new-start-failure", "stop-failure", "swap-failure" })
        {
            string root = Path.Combine(parent, scenario); Directory.CreateDirectory(root);
            string target = Path.Combine(root, "current"), stage = Path.Combine(root, "stage"), backup = Path.Combine(root, "backup");
            bool existing = scenario != "new" && scenario != "new-start-failure";
            var registration = new Registration { present = existing, active = existing && scenario != "rollback-stopped" };
            if (existing) { Directory.CreateDirectory(target); File.WriteAllText(Path.Combine(target, "version"), "old"); }
            if (scenario != "swap-failure") { Directory.CreateDirectory(stage); File.WriteAllText(Path.Combine(stage, "version"), "new"); }
            registration.failStart = scenario.StartsWith("rollback-", StringComparison.Ordinal) || scenario == "new-start-failure";
            registration.failStop = scenario == "stop-failure";
            bool failed = false;
            try { BrokerTransaction.Apply(target, stage, backup, registration); } catch (IOException) { failed = true; }
            bool expectedFailure = scenario != "new" && scenario != "update";
            Assert(failed == expectedFailure, "Unexpected transaction result: " + scenario);
            if (failed)
            {
                Assert(registration.present == existing, "Registration not restored: " + scenario);
                Assert(Directory.Exists(target) == existing, "Installation presence not restored: " + scenario);
                if (existing) Assert(File.ReadAllText(Path.Combine(target, "version")) == "old", "Old bytes not restored: " + scenario);
                Assert(registration.active == (existing && scenario != "rollback-stopped"), "Prior running state not restored: " + scenario);
                if (scenario == "stop-failure") Assert(registration.starts == 0, "Failed stop must not restart an already-running old service.");
            }
            else Assert(registration.present && registration.active && File.ReadAllText(Path.Combine(target, "version")) == "new", "Successful deployment not active.");
        }
    }
    private static void Payloads(string parent)
    {
        string file = Path.Combine(parent, "payload"); File.WriteAllText(file, "untrusted");
        bool rejected = false;
        try { using (BrokerDeployment.VerifiedFile(file, 9, new String('0', 64))) { } } catch (InvalidDataException) { rejected = true; }
        Assert(rejected, "Wrong payload hash accepted.");
        string hash;
        using (var sha = System.Security.Cryptography.SHA256.Create()) hash = BitConverter.ToString(sha.ComputeHash(Encoding.UTF8.GetBytes("untrusted"))).Replace("-", "");
        using (BrokerDeployment.VerifiedFile(file, 9, hash))
        {
            bool writeBlocked = false;
            try { File.WriteAllText(file, "replacement"); } catch (IOException) { writeBlocked = true; }
            Assert(writeBlocked, "Verified source could be changed while copying.");
        }
        Assert(BrokerDeployment.Owner("S-1-5-21-1-2-3-1001").Value == "S-1-5-21-1-2-3-1001", "Local account rejected.");
        Assert(BrokerDeployment.Owner("S-1-12-1-1-2-3-4").Value == "S-1-12-1-1-2-3-4", "Cloud account rejected.");
        foreach (string sid in new[] { "S-1-1-0", "S-1-5-18", "S-1-5-32-545", "invalid" })
        {
            rejected = false; try { BrokerDeployment.Owner(sid); } catch (ArgumentException) { rejected = true; }
            Assert(rejected, "Broad or invalid owner accepted.");
        }
        Assert(BrokerDeployment.Dispatch(new[] { "--install", "missing-consent" }) == 2, "Missing consent accepted.");
        Assert(BrokerDeployment.Dispatch(new[] { "--prepare-maintenance", "missing-consent" }) == 2, "Maintenance without consent accepted.");
    }
    private static int Main(string[] args)
    {
        string root = Path.Combine(Path.GetTempPath(), "freergb-deployment-test-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            Transactions(root); Payloads(root);
            if (args.Length == 2)
            {
                string library = Path.Combine(root, "runtime");
                BrokerDeployment.StageRuntime(args[0], args[1], library);
                Assert(File.Exists(Path.Combine(library, "LibreHardwareMonitorLib.dll")), "Pinned runtime missing.");
                Assert(File.Exists(Path.Combine(library, "PawnIO.Modules-LGPL-2.1.txt")), "Module license missing.");
                Assert(!File.Exists(Path.Combine(library, "LibreHardwareMonitor.exe")), "GUI executable staged.");
            }
        }
        finally { Directory.Delete(root, true); }
        Console.WriteLine("PASS: deployment disk transactions, failed-start rollback, preserved running state, immutable verified payloads, owner/consent gates. No SCM or hardware access.");
        return 0;
    }
}
