import { showMainWindow } from "../lib/ipc";

// The menu bar / tray popover (board D). Screens fill this in later.
export function TrayApp() {
  return (
    <div className="flex h-screen flex-col gap-3 p-4">
      <p className="font-bold">Wardwatch</p>
      <p className="text-sm opacity-80">Nothing needs you right now.</p>
      <button
        type="button"
        className="mt-auto rounded-lg border border-current px-3 py-2 text-sm"
        onClick={() => void showMainWindow()}
      >
        Open Wardwatch
      </button>
    </div>
  );
}
