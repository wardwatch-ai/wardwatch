// Entry point for the fix screen. The fix screen thread replaces this file with
// the real screen (plain-language explanation, 1-click update, live progress).
// Routes: /fix/:source/:id for one program, /fix/tune-up for everything waiting.
import { Link, useParams } from "react-router";
import { useDevice } from "../../lib/data";

export function Fix() {
  const { source, id } = useParams();
  const { programs } = useDevice();
  const program = programs?.find((p) => p.source === source && p.id === id);
  const title = source === undefined ? "Sunday tune-up" : (program?.name ?? id ?? "Update");
  return (
    <section aria-labelledby="fix-title" className="flex flex-col gap-3">
      <h1 id="fix-title" className="text-3xl">
        {title}
      </h1>
      <p className="m-0 text-text-muted">The fix screen is coming next.</p>
      <Link to="/">Back to Home</Link>
    </section>
  );
}
