// Transport acceptance fixture: no LHM, drivers or hardware access.
using System;
using System.IO;
using System.Threading;
internal static class Fixture
{
    private static void Main(string[] args)
    {
        string directory = args[1];
        if (directory.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase)) directory = @"\\" + directory.Substring(8);
        else if (directory.StartsWith(@"\\?\", StringComparison.Ordinal)) directory = directory.Substring(4);
        string scenario = File.ReadAllText(Path.Combine(directory, "scenario.txt"));
        while (Console.ReadLine() != null)
        {
            if (scenario == "hang" || scenario == "cancel") { Thread.Sleep(30000); return; }
            if (scenario == "exit") return;
            if (scenario == "bad") Console.WriteLine("{\"version\":9}");
            else if (scenario == "oversized") Console.WriteLine(new string('x', 300000));
            else Console.WriteLine("{\"version\":1,\"status\":\"ready\",\"message\":\"Transport fixture only\",\"hardware\":[{\"id\":\"fixture\",\"name\":\"Fixture\",\"sensors\":[{\"id\":\"rpm\",\"name\":\"Fixture RPM\",\"kind\":\"fan\",\"value\":900,\"age_ms\":0,\"control_available\":false}]}]}");
            Console.Out.Flush();
        }
    }
}
