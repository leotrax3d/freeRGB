// Disk/process tests only. No privileged directory, service, driver or hardware.
using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Threading;

internal static class NativeJournalTests
{
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    private static void Reject(Action action, string message)
    {
        try { action(); } catch (InvalidOperationException) { return; } catch (InvalidDataException) { return; } catch (IOException) { return; } catch (UnauthorizedAccessException) { return; } catch (ArgumentException) { return; }
        throw new Exception(message);
    }
    private static NativeControlIdentity Identity() { return new NativeControlIdentity("/synthetic/board","/synthetic/control/7",false); }
    internal static void Child(string root)
    {
        using (var journal = NativeControlJournal.ForTest(root))
        {
            var port = JournaledNativeControl.ForTest(new SessionPort { Floor = 40 },journal,Identity());
            port.Acquire();
            Console.WriteLine("journal-intent-durable"); Console.Out.Flush();
            Console.ReadLine(); // Parent kills this process before any release.
        }
    }
    internal static void Run(Func<ControlSink,NativeControl> create)
    {
        string root = Path.Combine(Path.GetTempPath(),"freergb-native-journal-"+Guid.NewGuid().ToString("N"));
        string path = Path.Combine(root,"mainboard-control-recovery.json");
        try
        {
            using (var journal = NativeControlJournal.ForTest(root))
            {
                string concurrentId = null;
                Exception concurrentError = null;
                using (var entered = new ManualResetEvent(false))
                using (var completed = new ManualResetEvent(false))
                {
                    Thread contender = null;
                    try
                    {
                        journal.Maintenance(delegate {
                            contender = new Thread(delegate() {
                                try
                                {
                                    using (var other = NativeControlJournal.ForTest(root))
                                    { entered.Set(); concurrentId = other.Reserve(Identity()); }
                                }
                                catch (Exception error) { concurrentError = error; }
                                finally { completed.Set(); }
                            });
                            contender.Start();
                            Assert(entered.WaitOne(3000),"Concurrent acquisition did not start.");
                            Assert(!completed.WaitOne(150),"Acquisition escaped maintenance exclusion.");
                            Assert(journal.Pending() == null,"Concurrent acquisition wrote while maintenance held the mutex.");
                        });
                    }
                    finally { if (contender != null) Assert(contender.Join(6000),"Concurrent acquisition failed to finish after maintenance."); }
                    Assert(concurrentError == null && concurrentId != null,"Acquisition did not resume after maintenance exclusion ended.");
                    journal.Complete(concurrentId);
                }
                var native = new SessionPort { Floor = 40 };
                native.BeforeAcquire = delegate { Assert(journal.Pending() != null,"Native acquisition preceded durable intent."); };
                var port = JournaledNativeControl.ForTest(native,journal,Identity());
                port.Acquire();
                string initial = File.ReadAllText(path);
                bool maintained = false;
                Reject(delegate { journal.Maintenance(delegate { maintained = true; }); },"Pending intent permitted maintenance.");
                Assert(!maintained && File.ReadAllText(path) == initial,"Refused maintenance mutated service or journal.");
                Reject(delegate { journal.Recover(delegate { return false; }); },"Unverified recovery cleared the intent.");
                Assert(File.ReadAllText(path) == initial,"Unverified recovery mutated durable evidence.");
                Reject(delegate { journal.Recover(delegate { throw new IOException("Synthetic recovery failure."); }); },"Throwing recovery returned success.");
                Assert(File.ReadAllText(path) == initial,"Failed recovery cleared durable evidence.");
                port.SetDuty(40);
                Assert(File.ReadAllText(path) == initial,"Heartbeat duty rewrote the recovery journal.");
                using (var another = NativeControlJournal.ForTest(root))
                    Reject(delegate { another.Reserve(new NativeControlIdentity("other","other",false)); },"Unresolved intent permitted another channel.");
                Reject(delegate { journal.Complete(Guid.NewGuid().ToString("N")); },"Stale reservation cleared another intent.");
                native.FailRelease = true;
                Reject(port.Release,"Failed native restore cleared the journal.");
                Assert(journal.Pending() != null && native.Owned,"Restore failure lost ownership/intent.");
                native.FailRelease = false;
                port.Release(); port.Release();
                Assert(journal.Pending() == null && !native.Owned,"Successful restore did not clear intent.");

                Directory.CreateDirectory(path+".tmp");
                native = new SessionPort { Floor = 40 };
                port = JournaledNativeControl.ForTest(native,journal,Identity());
                Reject(port.Acquire,"Journal write failure did not block native acquisition.");
                Assert(native.Writes.Length == 0,"Native acquisition occurred despite failed durable write.");
                Directory.Delete(path+".tmp");

                native = new SessionPort { Floor = 40, FailAcquire = true };
                port = JournaledNativeControl.ForTest(native,journal,Identity());
                Reject(port.Acquire,"Partial native acquisition returned success.");
                Assert(journal.Pending() != null && native.Owned,"Partial acquisition lost journal/ownership.");
                port.Release();

                native = new SessionPort { Floor = 40 };
                port = JournaledNativeControl.ForTest(native,journal,Identity());
                port.Acquire();
                Directory.CreateDirectory(path+".tmp");
                Reject(port.Release,"Failed durable clear returned success.");
                Assert(!native.Owned && journal.Pending() != null,"Failed clear was hidden or falsely retained native mode.");
                int writes = native.Writes.Length;
                Directory.Delete(path+".tmp");
                port.Release();
                Assert(journal.Pending() == null && native.Writes.Length == writes,"Clear retry performed another native restore.");

                var sink = new ControlSink();
                NativeControl real = create(sink);
                NativeControlIdentity identity = real.Identity();
                Assert(identity.HardwareId == "/synthetic/board" && identity.SensorId == "/synthetic/control/7" && !identity.Pump,"Journal identity did not derive from the selected native sensor/hardware.");
                var wrapped = JournaledNativeControl.ForTest(real,journal,identity);
                long now = 0;
                var session = NativeControlSession.ForTest(wrapped,delegate { return now; });
                session.Execute(NativeSessionTests.Command(session.View(),"set",40));
                now = 1000; session.Poll();
                Assert(journal.Pending() != null && real.Owned && sink.Writes[2] == 100,"Lease emergency cleared its durable recovery intent.");
                wrapped.Release();
                Assert(journal.Pending() == null && sink.Writes[3] == null,"Explicit journaled restoration failed.");

                foreach (string invalid in new[] { "not-json", "{\"version\":2,\"reservation\":null}", new string('x',16385) })
                {
                    File.WriteAllText(path,invalid);
                    native = new SessionPort { Floor = 40 };
                    port = JournaledNativeControl.ForTest(native,journal,Identity());
                    Reject(port.Acquire,"Invalid/oversized journal permitted native acquisition.");
                    Assert(native.Writes.Length == 0,"Invalid journal reached I/O.");
                    Reject(delegate { journal.Maintenance(delegate { maintained = true; }); },"Corrupt journal permitted maintenance.");
                    Assert(!maintained,"Corrupt journal reached maintenance callback.");
                }
            }
            string crashed = Path.Combine(root,"crashed-worker");
            using (var process = new Process { StartInfo = new ProcessStartInfo(Assembly.GetExecutingAssembly().Location,"--journal-child \""+crashed+"\"") {
                UseShellExecute = false, CreateNoWindow = true, RedirectStandardInput = true, RedirectStandardOutput = true
            } })
            {
                process.Start();
                try
                {
                    var read = process.StandardOutput.ReadLineAsync();
                    Assert(read.Wait(5000) && read.Result == "journal-intent-durable","Synthetic worker did not durably arm.");
                    process.Kill(); Assert(process.WaitForExit(3000),"Synthetic worker could not be reaped.");
                    using (var journal = NativeControlJournal.ForTest(crashed))
                    {
                        Assert(journal.Pending() != null,"Worker crash lost its control intent.");
                        bool maintained = false;
                        Reject(delegate { journal.Maintenance(delegate { maintained = true; }); },"Killed-worker orphan permitted maintenance.");
                        Assert(!maintained,"Orphan maintenance reached service mutation.");
                        var native = new SessionPort { Floor = 40 };
                        var port = JournaledNativeControl.ForTest(native,journal,Identity());
                        Reject(port.Acquire,"Restart resumed an orphaned channel.");
                        Assert(native.Writes.Length == 0,"Restart reached native I/O before resolving the old intent.");
                    }
                }
                finally { if (!process.HasExited) { process.Kill(); process.WaitForExit(3000); } }
            }
        }
        finally { if (Directory.Exists(root)) Directory.Delete(root,true); }
        Console.WriteLine("PASS: write-ahead journal, failed writes/restores/clear retries, real-LHM session bridge and killed-worker restart refusal. Test-directory storage and synthetic callbacks only.");
    }
}
