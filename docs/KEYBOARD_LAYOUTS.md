# Shared keyboard layouts

`freergb_module_api::layout` provides `KeyboardLayoutManager`, a validated
catalog for A5 keyboard layouts. It is a pure data library with no device I/O,
new protocol messages, or changes to the API version. Existing modules continue
to return `Capabilities.layout: freergb_core::LedLayout`.

A `KeyboardLayout` separates four things:

- `keys` defines frame order, stable physical key IDs and regional display names.
- Key `position` and `width`/`height` define the visual grid in `(column, row)` order.
- `matrix` defines the controller's row-major matrix. Cells contain frame indices;
  `None` represents a hole. Every key appears exactly once.
- `wire_indices` maps each frame index to its device-specific LED address.
  Addresses can be sparse; zero and 65535 are ordinary addresses, not sentinels.

Visual positions, matrix cells and wire addresses need not have the same order.
Wide keys occupy one visual position and one matrix cell; other matrix cells
covered by them are holes. This API describes LED centres, not keycap outlines.
Matrix dimensions are nonzero and contain at most `MAX_LEDS` cells; key counts
are in `1..=MAX_LEDS`. Key IDs, display names, positions and wire addresses must
each be unique. IDs and names must be nonblank, and positions must fit the grid.

Modules construct model-specific tables, call `manager.register(layout)?`, and
select an exact catalog ID with `manager.get(id)`. Duplicate IDs are rejected
without replacement. Iteration is ordered by ID. There is no automatic regional
fallback and no built-in device address table: those require verified model data.
`PhysicalLayout` supports ANSI, ISO, JIS and custom arrangements. `region` is a
module-chosen label such as `en-US`; it does not infer a physical arrangement.

Use `regional_variant(id, region, &[("KeyY", "Z"), ("KeyZ", "Y")])?` to derive
regional names while retaining stable physical IDs, geometry and addressing.
For a physical change such as ANSI to ISO, supply a separate complete layout
with the correct extra key, geometry, matrix and wire indices. Name overrides
reject unknown or repeated key IDs and validate the final set of names.

After selection, `layout.led_layout()?` produces the existing capability layout
in frame order. A frame encoder can pair `colors[i]` with `layout.wire_indices[i]`,
or use `wire_index_at(row, column)` for a matrix-based protocol. Out-of-grid
coordinates and holes return `None`. Encoding packet headers, channels, RGB
planes and report lengths remains the module's responsibility. Public structs
support serde; deserialization alone does not validate them. Registration and
`led_layout()` validate, and direct consumers must call `validate()` first.

The unit fixtures deliberately use different visual, matrix and wire orders,
including holes and regional Y/Z name swaps. They are synthetic. The Corsair
keyboard STREAM `0x7F` proof from Phase A still requires a separately scoped
device-module port with verified model/region tables; this change supplies its
shared mapping API and does not claim hardware coverage.
