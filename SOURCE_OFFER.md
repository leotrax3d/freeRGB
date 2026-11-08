# Source code for downloadable modules (DRAFT)

> **DRAFT for lawyer review.** This text is a proposal prepared by engineering.
> It is not approved legal wording and makes no statement that any obligation
> is met. Placeholders in `[BRACKETS]` need an owner decision. See
> `docs/legal/OWNER_DECISIONS.md` (D2).

freeRGB hardware modules are separate programs that the application can download
on request. Modules ported from OpenRGB and similar projects are licensed under
the GNU General Public License, version 2 or (at your option) any later version
(GPL-2.0-or-later). Each module's license is stated in its `module-info.json`
and its `COPYING` file, and its origin in its `PROVENANCE.md`.

The freeRGB core application is not licensed under the GPL (see `NOTICE`).

## Where the source is

For every module binary package

    freergb-module-<module-id>-<version>-<target>.zip

the matching complete corresponding source is published at the same download
location (the `modules` release of https://github.com/leotrax3d/freeRGB) as

    freergb-module-<module-id>-<version>-source.zip

The source archive is produced by the same CI workflow from the same commit and
contains the module sources, the workspace files needed to build it and the
license texts. Module sources are also in the repository under
`modules/<module-id>/`.

## Written offer

[OWNER/LAWYER TO CONFIRM WORDING AND CONTACT]

If you received a module binary and cannot obtain the matching source archive
from the location above, the publisher will provide a copy of the complete
corresponding source for that module on request, for a charge no more than the
cost of physically performing the distribution, for at least three years after
the module version was last distributed. Request it at: [CONTACT ADDRESS OR
ISSUE TRACKER URL - TO BE DECIDED BY THE OWNER].

Please state the module id, version and the file name you received.

## Open points

- Whether the module/core boundary is a separate-works boundary is a lawyer
  question (D2/D3) and is not decided here.
- Only modules listed in `packaging/modules/release-allowlist.txt` (when that
  file is present on the release branch) are intended for publication; the
  per-module provenance gaps are listed in
  `docs/legal/MODULE_PROVENANCE_GAPS.md`.
