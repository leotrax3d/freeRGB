#!/bin/sh
# Checks that the hwmon rule in packaging/linux/60-freergb.rules matches only
# pwmN and pwmN_enable. Uses the glob list from the rule against a fixture dir.
set -eu
rules="$(dirname "$0")/../packaging/linux/60-freergb.rules"
dir="$(mktemp -d)"; trap 'rm -rf "$dir"' EXIT
for n in pwm1 pwm1_enable pwm1_mode pwm1_freq pwm1_auto_point1_pwm pwm10 pwm10_enable pwm1_auto_channels_temp fan1_input; do : > "$dir/$n"; done
globs=$(grep -o 'for f in [^;]*; do' "$rules" | sed 's/^for f in //; s/ ; do$//; s/ *; do$//')
matched=""
globs=$(printf '%s' "$globs" | sed "s#/sys%p#$dir#g")
for f in $globs; do [ -f "$f" ] && matched="$matched $(basename "$f")"; done
want=" pwm1 pwm10 pwm1_enable pwm10_enable"
got=" $(printf '%s\n' $matched | sort | tr '\n' ' ')"
sorted_want=" $(printf '%s\n' $want | sort | tr '\n' ' ')"
[ "$got" = "$sorted_want" ] || { echo "FAIL: matched:$got"; exit 1; }
echo "ok: $got"
