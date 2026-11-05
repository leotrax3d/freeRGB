// Snapshot update budget (owner acceptance 2026-11-05, item 8): the motherboard
// SuperIO tree (fans, controls, board temperatures) is updated on every
// snapshot; slower hardware (CPU, GPU) only when its own interval is due. The
// interval (1..2 s) grows with that hardware's measured update time, so one slow sensor
// can no longer push the whole reply past the broker deadline. Reused values
// keep their true age, so consumers see them as older, never as fresh.
using System;
using System.Collections.Generic;

internal sealed class UpdateCadence
{
    internal const long MinimumIntervalMs = 1000;
    internal const long MaximumIntervalMs = 2000;
    /// <summary>Update time above which a hardware item is reported as slow.</summary>
    internal const long SlowMs = 500;
    private sealed class Entry { internal long At = -1; internal long DurationMs; }
    private readonly Dictionary<string, Entry> entries = new Dictionary<string, Entry>();

    /// <summary>Interval: twice the last update time, 1..2 s, so reused CPU/GPU temperatures stay younger than the 3 s freshness limit of curves.</summary>
    internal long IntervalMs(string key)
    {
        Entry entry;
        long last = entries.TryGetValue(key, out entry) ? entry.DurationMs : 0;
        return Math.Max(MinimumIntervalMs, Math.Min(MaximumIntervalMs, 2 * last));
    }

    /// <summary>Whether hardware should be updated now. Fast hardware always is.</summary>
    internal bool Due(string key, bool fast, long nowMs)
    {
        Entry entry;
        if (fast || !entries.TryGetValue(key, out entry) || entry.At < 0) return true;
        return nowMs - entry.At >= IntervalMs(key);
    }

    /// <summary>Records an update that started at <paramref name="nowMs"/>; the interval counts from its end.</summary>
    internal void Record(string key, long nowMs, long durationMs)
    {
        Entry entry;
        if (!entries.TryGetValue(key, out entry)) { entry = new Entry(); entries.Add(key, entry); }
        entry.At = nowMs + Math.Max(0, durationMs);
        entry.DurationMs = Math.Max(0, durationMs);
    }

    /// <summary>Time since the last update of this hardware started (conservative; 0 if never).</summary>
    internal long AgeMs(string key, long nowMs)
    {
        Entry entry;
        return entries.TryGetValue(key, out entry) && entry.At >= 0 ? Math.Max(0, nowMs - (entry.At - entry.DurationMs)) : 0;
    }

    /// <summary>Last measured update time per hardware, slowest first, above <see cref="SlowMs"/>.</summary>
    internal List<KeyValuePair<string, long>> Slow()
    {
        var slow = new List<KeyValuePair<string, long>>();
        foreach (var pair in entries) if (pair.Value.DurationMs > SlowMs) slow.Add(new KeyValuePair<string, long>(pair.Key, pair.Value.DurationMs));
        slow.Sort(delegate (KeyValuePair<string, long> a, KeyValuePair<string, long> b) { return b.Value.CompareTo(a.Value); });
        return slow;
    }
}
