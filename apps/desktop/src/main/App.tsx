import { NavLink, Route, Routes } from "react-router";
import { useAppInfo } from "../lib/useAppInfo";
import { tabs } from "./routes";

function Placeholder({ title }: { title: string }) {
  return (
    <section aria-labelledby="screen-title" className="p-8">
      <h1 id="screen-title" className="text-2xl font-bold">
        {title}
      </h1>
      <p className="mt-2 opacity-70">This screen is not built yet.</p>
    </section>
  );
}

export function App() {
  const { data: info } = useAppInfo();

  return (
    <div className="flex min-h-screen flex-col">
      <header className="flex items-center gap-6 border-b border-black/10 px-6 py-3 dark:border-white/10">
        <span className="font-bold">Wardwatch</span>
        <nav aria-label="Main" className="flex gap-4">
          {tabs.map((tab) => (
            <NavLink
              key={tab.path}
              to={tab.path}
              end
              className={({ isActive }) => (isActive ? "font-semibold underline" : "opacity-80")}
            >
              {tab.label}
            </NavLink>
          ))}
        </nav>
      </header>
      <main className="flex-1">
        <Routes>
          {tabs.map((tab) => (
            <Route key={tab.path} path={tab.path} element={<Placeholder title={tab.label} />} />
          ))}
          <Route path="*" element={<Placeholder title="Home" />} />
        </Routes>
      </main>
      <footer className="px-6 py-2 text-xs opacity-60">
        {info ? `${info.name} ${info.version} on ${info.os}` : "Loading"}
      </footer>
    </div>
  );
}
