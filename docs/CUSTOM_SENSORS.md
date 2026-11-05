# Custom Celsius sources

Cooling → **Custom temperature sources** builds a graph from cached temperature
readings. Add a real input, then corrections, combinations or a rolling average.
Each node has a stable name; Advanced JSON allows custom names and ordered inputs.
Saved derived readings are evaluated by the daemon, independently of either GUI.
There is no hardware channel binding yet.

Save writes the graph under `sensors` in `%APPDATA%\freeRGB\control.json`
(or `$XDG_CONFIG_HOME/freergb/control.json`). The graph has `version: 1`;
missing version defaults to 1, unknown versions are rejected. No new GUI config
file is created. Save writes the daemon's control library atomically. Reload replaces unsaved
edits with the saved graph. Existing control files without a sensor graph still
load; failed validation/storage does not replace saved state. Average histories
are memory-only in the daemon, reset after restart/definition changes, and are not
persisted. Both GUIs consume the same readings through JSON-RPC.

Native source identifiers encode the device/sensor pair as a JSON array string
to avoid ambiguous concatenation. The source list contains only connected cached
Celsius sensors, including LHM/hwmon CPU, GPU, board and storage readings and
`file:<name>` text-file sensors (`COOLING_SOURCES_GPU.md`).

Definitions:

```json
{
  "version": 1,
  "nodes": {
    "source": {"type":"input","source":"[\"device-id\",\"temperature:4\"]"},
    "corrected": {"type":"offset","source":"source","amount":2.0,"proportional":false},
    "combined": {"type":"mix","sources":["source","corrected"],"function":"maximum"},
    "smoothed": {"type":"time_average","source":"combined","window_ms":10000}
  }
}
```

The input identifier above is a placeholder, not a measurement. Combination
functions are maximum, minimum, average, sum and ordered subtraction. Offsets can
be additive Celsius or proportional percent. All required inputs must be present:
a missing hot source cannot silently disappear from a maximum. Native readings
older than three seconds, future timestamps, nonfinite values and results outside
-100..200 C produce an unavailable result. These sanity bounds are not safe
operating limits for a particular component. Output timestamps preserve the oldest
dependency; transformations never manufacture freshness.

Rolling averages are time-weighted using held samples, over available history in
the requested 1..600-second window. Warm-up uses available data. Histories retain
at most 600 samples per averaging node, so high-frequency streams can have shorter
effective coverage. Missing inputs/large gaps, clock rewind and definition changes
reset history. Repeated evaluations at the same source timestamp do not add samples.

Graphs are limited to 128 nodes, 32 dependencies per combination and depth 32.
Missing references, cycles and invalid parameters are rejected. Unavailable
readings carry a null value and an error; no fan/pump command follows.
The editor displays saved-graph results; unsaved edits do not change evaluation.
