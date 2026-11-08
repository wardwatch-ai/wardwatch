// Settings: this copy's version, and which release channel updates come from.
// Opened from the account menu; the five tabs stay as designed.
import { Button, Card, Eyebrow } from "@wardwatch/ui";
import { useState } from "react";
import { useSaveSettings, useSettings } from "../../lib/data";
import { buildDate, releaseLabel } from "../../lib/format";
import type { AppInfo, ReleaseChannel } from "../../lib/ipc";
import { useAppInfo } from "../../lib/useAppInfo";
import { Failed, Loading } from "../parts";

const channels: { id: ReleaseChannel; label: string; hint: string }[] = [
  {
    id: "mainline",
    label: "Mainline (recommended)",
    hint: "Tested, numbered releases.",
  },
  {
    id: "nightly",
    label: "Nightly",
    hint: "New features first, built every night. It may have bugs.",
  },
];

const channelName: Record<AppInfo["channel"], string> = {
  mainline: "Mainline",
  nightly: "Nightly",
  dev: "Local build",
};

function ThisApp({ info, chosen }: { info: AppInfo; chosen: ReleaseChannel }) {
  const details = [
    channelName[info.channel],
    info.built && `built ${buildDate(info.built)}`,
  ].filter(Boolean);
  return (
    <Card className="flex flex-col gap-2">
      <Eyebrow>This app</Eyebrow>
      <div className="text-xl font-semibold">
        {info.name} {releaseLabel(info)}
      </div>
      <div className="text-sm text-text-muted">
        {details.join(" · ")}
        {info.commit && (
          <>
            {" · "}
            <code title="The source code this copy was built from">{info.commit}</code>
          </>
        )}
      </div>
      <p className="m-0 mt-1">
        {info.os === "linux"
          ? "On Linux, Wardwatch updates through your package manager (apt or dnf), like your other software."
          : "Wardwatch will update itself, asking you first, from its first signed release. This copy doesn't update itself yet."}
      </p>
      {info.channel !== "dev" && info.channel !== chosen && (
        <p className="m-0 text-sm text-text-muted">
          You'll move to {channelName[chosen].toLowerCase()} with the next update.
        </p>
      )}
    </Card>
  );
}

export function Settings() {
  const { data: info, error: infoError } = useAppInfo();
  const { data: settings, error } = useSettings();
  const save = useSaveSettings();
  const [confirming, setConfirming] = useState(false);
  const failure = error ?? infoError;
  if (failure) return <Failed error={failure} />;
  if (!settings || !info) return <Loading />;

  const choose = (channel: ReleaseChannel) => {
    setConfirming(false);
    if (channel !== settings.channel) save.mutate({ ...settings, channel });
  };

  return (
    <div className="flex max-w-[720px] flex-col gap-5">
      <h1 className="text-3xl">Settings</h1>
      <ThisApp info={info} chosen={settings.channel} />

      <Card className="flex flex-col gap-3">
        <Eyebrow id="channel-label">Release channel</Eyebrow>
        <div role="radiogroup" aria-labelledby="channel-label" className="flex flex-col gap-2">
          {channels.map((c) => (
            <label
              key={c.id}
              className="flex min-h-11 cursor-pointer items-start gap-3 rounded-[10px] border border-line p-3 has-checked:border-text"
            >
              <input
                type="radio"
                name="channel"
                value={c.id}
                checked={(confirming ? "nightly" : settings.channel) === c.id}
                onChange={() => {
                  if (c.id === "nightly") setConfirming(true);
                  else choose(c.id);
                }}
                className="mt-1 accent-text"
              />
              <span>
                <span className="block font-semibold">{c.label}</span>
                <span className="block text-sm text-text-muted">{c.hint}</span>
              </span>
            </label>
          ))}
        </div>
        {confirming && settings.channel !== "nightly" && (
          <Card tone="inset" as="div" role="alertdialog" aria-label="Switch to nightly?">
            <p className="m-0">
              Nightly builds get changes the day they're made, before they're fully tested.
              Wardwatch still watches your software, but parts of it may break.
            </p>
            <div className="mt-3 flex flex-wrap gap-2">
              <Button
                variant="brand"
                onClick={() => {
                  choose("nightly");
                }}
              >
                Switch to nightly
              </Button>
              <Button
                variant="quiet"
                onClick={() => {
                  setConfirming(false);
                }}
              >
                Stay on mainline
              </Button>
            </div>
          </Card>
        )}
        {save.isError && <p role="alert">Couldn't save your choice: {save.error.message}</p>}
      </Card>
    </div>
  );
}
