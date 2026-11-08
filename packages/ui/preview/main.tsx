import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "../src/styles/index.css";
import {
  AccountPill,
  ActGlyph,
  BandBar,
  Button,
  Card,
  CheckGlyph,
  Chip,
  Eyebrow,
  NavTab,
  OkGlyph,
  SegmentedControl,
  setTheme,
  StatusPill,
  SubscoreBar,
  TopBar,
  type ThemePreference,
} from "../src";

// Sample data from the design spec ("Sample data in the mocks").
function Gallery() {
  const initial = (new URLSearchParams(location.search).get("theme") as ThemePreference) ?? "system";
  const [theme, setThemeState] = useState<ThemePreference>(initial);
  const [period, setPeriod] = useState<"30d" | "month" | "all">("30d");
  useEffect(() => setTheme(theme), [theme]);
  const row = { display: "flex", flexWrap: "wrap", gap: 12, alignItems: "center" } as const;
  return (
    <>
      <TopBar
        beams
        nav={["Home", "Score", "Groups", "Apps", "Update plan"].map((t, i) => (
          <NavTab key={t} href={`#${t}`} current={i === 0}>
            {t}
          </NavTab>
        ))}
        account={<AccountPill name="Matt" score={84} />}
      />
      <main style={{ maxWidth: 1160, margin: "0 auto", padding: "32px 28px", display: "grid", gap: 20 }}>
        <div style={row}>
          <Eyebrow>Theme</Eyebrow>
          <SegmentedControl
            label="Theme"
            value={theme}
            onChange={setThemeState}
            options={[
              { value: "system", label: "System" },
              { value: "light", label: "Light" },
              { value: "dark", label: "Dark" },
            ]}
          />
        </div>
        <Card tone="urgent" style={{ display: "grid", gap: 10 }}>
          <Eyebrow style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <ActGlyph /> Under attack now
          </Eyebrow>
          <h2>Update Chrome.</h2>
          <div>
            This is being exploited right now.
            <div style={{ color: "var(--ww-text-muted)" }}>1 minute to update. Tabs will come back.</div>
          </div>
          <div style={row}>
            <Button variant="act">Update now</Button>
            <Button variant="act" href="#fix">
              Fix now (link)
            </Button>
            <a href="#why">What's going on?</a>
          </div>
        </Card>
        <Card style={{ display: "grid", gap: 10 }}>
          <Eyebrow>Sunday tune-up</Eyebrow>
          <h2>Zoom, VLC and macOS 16.1.1</h2>
          <div style={row}>
            <Button variant="quiet" severity={2}>
              Update all 3
            </Button>
            <Button variant="quiet" severity={1}>
              Update now
            </Button>
            <Button variant="quiet">Release hold</Button>
            <Button variant="brand">Invite</Button>
            <StatusPill tone="ok">Up to date</StatusPill>
          </div>
        </Card>
        <Card tone="remote" style={{ display: "grid", gap: 10 }}>
          <strong>Update the iPhone</strong>
          <div>iOS 26.1.1 is ready. Open Settings on the iPhone, then General, then Software Update.</div>
          <div>
            <StatusPill tone="remote">Update on the iPhone</StatusPill>
          </div>
        </Card>
        <Card style={{ display: "grid", gap: 22 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 14 }}>
            <span className="ww-display" style={{ fontSize: 88, lineHeight: 1 }}>
              84
            </span>
            <span className="ww-display" style={{ fontSize: 28, color: "var(--ww-ok)" }}>
              Good
            </span>
          </div>
          <BandBar score={84} />
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))", gap: 16 }}>
            <Card tone="inset">
              <strong>How up to date you are · 90</strong>
              <SubscoreBar value={90} label="How up to date you are" />
            </Card>
            <Card tone="inset">
              <strong>How fast you fix · 78</strong>
              <SubscoreBar value={78} label="How fast you fix" />
            </Card>
          </div>
        </Card>
        <Card tone="ok" style={row}>
          <OkGlyph /> Family iMac all clear <CheckGlyph /> System current
        </Card>
        <div style={row}>
          <SegmentedControl
            label="Period"
            value={period}
            onChange={setPeriod}
            options={[
              { value: "30d", label: "Last 30 days" },
              { value: "month", label: "This month" },
              { value: "all", label: "Since joining" },
            ]}
          />
          <Chip on={false}>Relaxed</Chip>
          <Chip on>Balanced</Chip>
          <Chip on={false}>Eager</Chip>
        </div>
      </main>
    </>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Gallery />
  </StrictMode>,
);
