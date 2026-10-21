import { AppWindow, Bookmark, Trash2 } from "lucide-react";
import { ConfirmButton } from "@/components/confirm-button";
import { NameForm } from "@/components/name-form";
import { PageHeader, Section } from "@/components/page";
import { EmptyState } from "@/components/status";
import { Button } from "@/components/ui/button";
import { rpc, useDaemon, useSlices } from "@/lib/store/daemon";
import { useAction } from "@/lib/store/useAction";
import { cn } from "@/lib/utils";

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
            <li
              key={name}
              className={cn("flex items-center gap-3 px-4 py-2.5 transition-opacity", pending === name && "opacity-60")}
            >
              <Bookmark className="size-4 text-text-subtle" aria-hidden />
              <span className="flex-1 truncate text-body">{name}</span>
              <Button
                size="sm"
                variant="secondary"
                disabled={pending !== null}
                onClick={() => run(name, () => rpc("profiles.load", { name }), `Profile ${name} loaded`)}
              >
                {pending === name ? "Loading…" : "Load"}
              </Button>
              <ConfirmButton
                label={`Delete ${name}`}
                icon={<Trash2 />}
                disabled={pending !== null}
                onConfirm={() => run(name, () => rpc("profiles.delete", { name }), `Profile ${name} deleted`)}
              />
            </li>
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
