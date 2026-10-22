// Multi-channel journal/recovery tests. Temporary directories and synthetic
// lifetimes only; no service, driver or hardware.
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;

internal static class NativeMultiChannelTests
{
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action, string message)
    {
        try { action(); }
        catch (InvalidOperationException) { return; } catch (InvalidDataException) { return; } catch (ArgumentException) { return; }
        throw new Exception(message);
    }
    private static NativeControlIdentity Owned(int index, string worker)
    {
        var recovery = new Dictionary<string,object> { {"kind","lhm_fallback_v1"},{"service_identity","100:200"},{"worker_identity",worker} };
        return new NativeControlIdentity("/lpc/it8689e/0","/lpc/it8689e/0/control/" + index,false,recovery,"lhm_supported");
    }
    internal static void Run(Func<ControlSink,object> create)
    {
        string root = Path.Combine(Path.GetTempPath(),"freergb-multi-journal-"+Guid.NewGuid().ToString("N"));
        string path = Path.Combine(root,"mainboard-control-recovery.json");
        var living = new HashSet<string> { "11:1", "12:1", "13:1" };
        Func<string,bool> alive = delegate(string worker) { return living.Contains(worker); };
        try
        {
            using (var journal = NativeControlJournal.ForTest(root))
            {
                string first = journal.Reserve(Owned(0,"11:1"),alive);
                string text = File.ReadAllText(path);
                Assert(text.StartsWith("{\"version\":1,\"reservation\":{"),"Single channel did not keep the legacy journal shape.");
                string second = journal.Reserve(Owned(1,"12:1"),alive);
                string third = journal.Reserve(Owned(2,"13:1"),alive);
                Assert(File.ReadAllText(path).StartsWith("{\"version\":2,\"reservations\":[") && journal.PendingAll().Count == 3,"Concurrent channels were not all journaled.");
                Reject(delegate { journal.Reserve(Owned(1,"13:1"),alive); },"Same channel was reserved twice.");
                Reject(delegate { journal.Reserve(Owned(4,"14:1")); },"Reservation without liveness evidence accepted next to others.");
                bool maintained = false;
                Reject(delegate { journal.Maintenance(delegate { maintained = true; }); },"Pending channels permitted maintenance.");
                Assert(!maintained,"Maintenance callback ran with pending channels.");
                journal.Complete(second); // Release in any order.
                Reject(delegate { journal.Complete(second); },"A completed reservation was completed again.");
                Assert(journal.PendingAll().Count == 2,"Out-of-order release removed the wrong entries.");

                living.Remove("13:1"); // Worker of channel 2 crashed.
                Reject(delegate { journal.Reserve(Owned(5,"11:1"),alive); },"An orphaned channel did not block new reservations.");
                var restored = new List<string>();
                journal.Recover(delegate(Dictionary<string,object> entry) { restored.Add((string)entry["sensor_id"]); return true; },
                    delegate(Dictionary<string,object> entry) { return NativeControlJournal.OwnerAlive(entry,alive); });
                Assert(restored.Count == 1 && restored[0].EndsWith("/control/2"),"Recovery touched a live worker's channel or missed the orphan.");
                Assert(journal.PendingAll().Count == 1 && File.ReadAllText(path).StartsWith("{\"version\":1,"),"Recovery did not clear only the orphan.");
                journal.Complete(first);
                Assert(File.ReadAllText(path) == "{\"version\":1,\"reservation\":null}","Empty journal changed its legacy form.");

                // Partial recovery: one failing channel keeps only its own intent.
                journal.Reserve(Owned(0,"11:1"),alive);
                journal.Reserve(Owned(1,"12:1"),alive);
                living.Clear();
                Reject(delegate {
                    journal.Recover(delegate(Dictionary<string,object> entry) {
                        if (((string)entry["sensor_id"]).EndsWith("/control/1")) throw new IOException("Synthetic failed restore.");
                        return true;
                    });
                },"Failed recovery of one channel reported success.");
                var left = journal.PendingAll();
                Assert(left.Count == 1 && ((string)left[0]["sensor_id"]).EndsWith("/control/1"),"Failed recovery lost or kept the wrong intents.");
                journal.Recover(delegate { return true; });
                Assert(journal.Pending() == null,"Retry did not clear the remaining intent.");

                foreach (string invalid in new[] {
                    "{\"version\":2,\"reservations\":[]}",
                    "{\"version\":2,\"reservations\":[" + Entry("a",0) + "]}",
                    "{\"version\":2,\"reservations\":[" + Entry("a",0) + "," + Entry("a",0) + "]}",
                    "{\"version\":3,\"reservations\":[" + Entry("a",0) + "," + Entry("b",1) + "]}" })
                {
                    File.WriteAllText(path,invalid);
                    Reject(delegate { journal.PendingAll(); },"Invalid multi-channel journal accepted: " + invalid);
                    Reject(delegate { journal.Reserve(Owned(6,"11:1"),alive); },"Invalid multi-channel journal permitted acquisition.");
                }
                File.WriteAllText(path,"{\"version\":1,\"reservation\":null}");

                // Two real LHM Control adapters, concurrently journaled and released in reverse order.
                living.Add("21:1"); living.Add("22:1");
                var sinks = new[] { new ControlSink(), new ControlSink() };
                var ports = new JournaledNativeControl[2];
                for (int i = 0; i < 2; i++)
                {
                    var recovery = new Dictionary<string,object> { {"kind","lhm_fallback_v1"},{"service_identity","100:200"},{"worker_identity",(21+i)+":1"} };
                    var adapter = NativeControl.ForTest(create(sinks[i]),false,40,NativeTier.LhmSupported,false,null,recovery);
                    var identity = adapter.Identity();
                    ports[i] = JournaledNativeControl.ForTest(adapter,journal,new NativeControlIdentity(identity.HardwareId,identity.SensorId + "/" + i,false,identity.Recovery,identity.Tier),alive);
                    ports[i].Acquire();
                    ports[i].SetDuty(40 + 10*i);
                }
                Assert(journal.PendingAll().Count == 2,"Two adapters were not reserved concurrently.");
                ports[1].Release(); ports[0].Release();
                Assert(journal.Pending() == null && sinks[0].Writes[sinks[0].Writes.Count-1] == null && sinks[1].Writes[sinks[1].Writes.Count-1] == null,
                    "Reverse-order release did not return both channels and clear the journal.");
            }
        }
        finally { if (Directory.Exists(root)) Directory.Delete(root,true); }
        Console.WriteLine("PASS: multi-channel journal (legacy single shape, concurrent live owners, any-order release, orphan blocking, per-channel recovery and partial failure). Synthetic only.");
    }
    private static string Entry(string hardware, int index)
    {
        return "{\"id\":\"" + Guid.NewGuid().ToString("N") + "\",\"backend\":\"" + LhmPin.Backend + "\",\"hardware_id\":\"/" + hardware + "\",\"sensor_id\":\"/" + hardware + "/control/" + index + "\",\"pump\":false}";
    }
}
