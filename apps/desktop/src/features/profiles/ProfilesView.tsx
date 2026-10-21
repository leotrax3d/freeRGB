import { AppWindow, Bookmark, Trash2 } from "lucide-react";
import { ConfirmButton } from "@/components/confirm-button";
import { NameForm } from "@/components/name-form";
import { PageHeader, Section } from "@/components/page";
import { EmptyState } from "@/components/status";
import { Button } from "@/components/ui/button";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { cn } from "@/lib/utils";
import { forget, markLoaded, useProfileState } from "./activeProfile";

function ProfileRow({
  name,
  pending,
  onLoad,
  onDelete,
}: {
  name: string;
  pending: string | null;
  onLoad: () => void;
  onDelete: () => void;
}) {
  const state = useProfileState(name);
  return (
    <li
      aria-current={state === "active" ? "true" : undefined}
      className={cn(
        "relative flex items-center gap-3 px-4 py-2.5 transition-colors duration-(--duration-fast)",
        state && "bg-surface-raised/60",
        pending === name && "opacity-60",
      )}
    >
      {state && <span className={cn("absolute inset-y-2 left-0 w-1 rounded-r-full", state === "active" ? "bg-accent" : "bg-warning")} aria-hidden />}
      <Bookmark className={cn("size-4", state === "active" ? "fill-accent text-accent" : "text-text-subtle")} aria-hidden />
      <span className="truncate text-body">{name}</span>
      {state === "active" && <span className="rounded-full bg-accent/15 px-2 py-0.5 text-caption font-medium text-accent">Active</span>}
      {state === "modified" && (
        <span className="rounded-full bg-warning/15 px-2 py-0.5 text-caption font-medium text-[#fbbf24]" title="Loaded last, but the lighting has changed since">
          Modified
        </span>
      )}
      <span className="flex-1" />
      <Button size="sm" variant="secondary" disabled={pending !== null} onClick={onLoad}>
        {pending === name ? "Loading…" : state ? "Reload" : "Load"}
      </Button>
      <ConfirmButton label={`Delete ${name}`} icon={<Trash2 />} disabled={pending !== null} onConfirm={onDelete} />
    </li>
  );
}

export default function ProfilesView() {
  useSlices("profiles");
  const profiles = useDaemon((s) => s.profiles);
  const { pending, error, run } = useAction();

  if (profiles === null) {
    return (
      <>
        <PageHeader title="Profiles" />
        <EmptyState icon={<Bookmark />} title="Profiles are turned off">
          The service was started with <code className="font-mono text-caption">--no-state</code>, so nothing can be
          saved. Restart it without that option to use profiles.
        </EmptyState>
      </>
    );
  }

  return (
    <>
      <PageHeader title="Profiles" description="Snapshots of your effect and brightness. Load one to switch instantly." />

      <Section title="Save current lighting">
        <div className="max-w-md">
          <NameForm
            label="Profile name"
            placeholder="e.g. Evening"
            submit="Save profile"
            existing={profiles ?? []}
            busy={pending !== null}
            onSubmit={(name) => run(`save:${name}`, () => rpc("profiles.save", { name }), `Profile ${name} saved`)}
          />
        </div>
      </Section>

      <Section title="Saved profiles" aside={error && <span className="text-caption text-danger-text">{error}</span>}>
        {profiles?.length === 0 && <p className="text-small text-text-subtle">No profiles yet.</p>}
        <ul className="flex flex-col divide-y divide-border overflow-hidden rounded-card bg-surface-sunken">
          {profiles?.map((name) => (
            <ProfileRow
              key={name}
              name={name}
              pending={pending}
              onLoad={() => run(name, async () => {
                await rpc("profiles.load", { name });
                await markLoaded(name);
              }, `Profile ${name} loaded`)}
              onDelete={() => run(name, async () => {
                await rpc("profiles.delete", { name });
                forget(name);
              }, `Profile ${name} deleted`)}
            />
          ))}
        </ul>
      </Section>

      <Section title="Automatic switching">
        <div className="flex items-start gap-3 rounded-card border border-border px-4 py-3 text-small text-text-muted">
          <AppWindow className="mt-0.5 size-4 shrink-0 text-text-subtle" aria-hidden />
          <span>
            Application profiles (switch when a program comes to the foreground) still run in freeRGB Classic. They
            move here once that logic is shared between both apps (M5).
          </span>
        </div>
      </Section>
    </>
  );
}
