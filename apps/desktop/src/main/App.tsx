import { AccountPill, OkGlyph, ActGlyph, TopBar } from "@wardwatch/ui";
import { useEffect, useId, useRef, useState } from "react";
import { Link, Navigate, NavLink, Route, Routes, useNavigate } from "react-router";
import { accounts, useDevice, useDevices, useMe, useScore } from "../lib/data";
import { onNavigate } from "../lib/ipc";
import { useAppInfo } from "../lib/useAppInfo";
import { tabs } from "./routes";
import { Apps } from "./screens/Apps";
import { Fix } from "./screens/Fix";
import { Groups } from "./screens/Groups";
import { Home } from "./screens/Home";
import { Plan } from "./screens/Plan";
import { Score } from "./screens/Score";
import { Settings } from "./screens/Settings";
import { TuneUp } from "./screens/TuneUp";

const screens: Record<(typeof tabs)[number]["path"], React.ReactNode> = {
  "/": <Home />,
  "/score": <Score />,
  "/groups": <Groups />,
  "/apps": <Apps />,
  "/plan": <Plan />,
};

/** Account pill and its menu, which lists the account's devices and opens Settings. */
function Account() {
  const me = useMe().data;
  const score = useScore().data;
  const devices = useDevices().data ?? [];
  const [open, setOpen] = useState(false);
  const menuId = useId();
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: Event) => {
      if (
        e instanceof KeyboardEvent ? e.key === "Escape" : !root.current?.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("keydown", close);
    document.addEventListener("mousedown", close);
    return () => {
      document.removeEventListener("keydown", close);
      document.removeEventListener("mousedown", close);
    };
  }, [open]);

  if (!me) return null;
  const name = me.real_name ?? me.username ?? "You";
  return (
    <div ref={root} className="relative">
      <AccountPill
        name={name}
        score={score?.score ?? 0}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={() => {
          setOpen(!open);
        }}
      />
      {open && (
        <div
          id={menuId}
          role="dialog"
          aria-label="Account"
          className="ww-card ww-card-default absolute right-0 top-[52px] z-10 flex w-72 flex-col gap-2 shadow-lg"
        >
          <div>
            <div className="font-semibold">{name}</div>
            <div className="text-sm text-text-muted">
              Signed in · {devices.length} {devices.length === 1 ? "device" : "devices"} on this
              account
            </div>
          </div>
          <div className="ww-eyebrow mt-2">Your devices</div>
          <ul className="m-0 flex list-none flex-col gap-1 p-0">
            {devices.map((d) => (
              <li key={d.id} className="flex min-h-11 items-center gap-2.5">
                {d.status === "needs_you" ? <ActGlyph /> : <OkGlyph />}
                <span className="flex-1 font-semibold">{d.name}</span>
                <span className="text-sm text-text-muted">
                  {d.status === "needs_you"
                    ? "Needs you"
                    : d.status === "waiting"
                      ? "Waiting"
                      : "All clear"}
                </span>
              </li>
            ))}
          </ul>
          <Link
            to="/settings"
            onClick={() => {
              setOpen(false);
            }}
            className="mt-1 flex min-h-11 items-center border-t border-line pt-2 font-semibold text-text no-underline"
          >
            Settings
          </Link>
        </div>
      )}
    </div>
  );
}

export function App() {
  const { data: info, error } = useAppInfo();
  const navigate = useNavigate();
  const urgent = (useDevice().status?.attack.length ?? 0) > 0;

  // The tray opens screens here (Update now, Score, Update plan).
  // Subscribe once: navigate's identity changes with the location, and re-subscribing
  // over async IPC on every tab change could drop an event in the gap.
  const navigateRef = useRef(navigate);
  useEffect(() => {
    navigateRef.current = navigate;
  });
  useEffect(() => onNavigate((route) => void navigateRef.current(route)), []);

  return (
    <div className="flex min-h-screen flex-col">
      <TopBar
        beams={urgent}
        nav={tabs.map((tab) => (
          <NavLink key={tab.path} to={tab.path} end={tab.path === "/"} className="ww-tab">
            {tab.label}
          </NavLink>
        ))}
        account={<Account />}
      />
      <main className="mx-auto box-border w-full max-w-[1200px] flex-1 px-4 py-6 sm:px-7 sm:py-8">
        <Routes>
          {tabs.map((tab) => (
            <Route key={tab.path} path={tab.path} element={screens[tab.path]} />
          ))}
          <Route path="/settings" element={<Settings />} />
          <Route path="/fix/tune-up" element={<TuneUp />} />
          <Route path="/fix/:source/:id" element={<Fix />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
      <footer className="px-7 py-2 text-xs text-text-muted">
        {error
          ? `Could not reach the Wardwatch core: ${error.message}`
          : info
            ? `${info.name} ${info.version} on ${info.os}`
            : "Loading"}
        {accounts.preview && " · Score and groups show sample data until sign-in is live."}
      </footer>
    </div>
  );
}
