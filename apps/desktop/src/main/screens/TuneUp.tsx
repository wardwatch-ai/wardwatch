// The Sunday tune-up: everything waiting for its plan slot, each with its own
// fix screen. Doing them all in one go comes with the nudge engine.
import { Link } from "react-router";
import { useDevice } from "../../lib/data";
import { Failed, Loading, UpdateButton } from "../parts";

export function TuneUp() {
  const { status, error } = useDevice();
  if (error) return <Failed error={error} />;
  if (!status) return <Loading />;
  const waiting = [...status.urgent, ...status.waiting];
  return (
    <section aria-labelledby="tune-up-title" className="flex flex-col gap-3">
      <h1 id="tune-up-title" className="text-3xl">
        Sunday tune-up
      </h1>
      {waiting.length === 0 ? (
        <p className="m-0 text-text-muted">
          {status.checked ? "Nothing is waiting." : "Nothing has been checked yet."}
        </p>
      ) : (
        <ul className="m-0 flex list-none flex-col gap-2 p-0">
          {waiting.map((p) => (
            <li key={p.key} className="flex items-center justify-between gap-3">
              <span>
                {p.name} <span className="text-text-muted">{p.version}</span>
              </span>
              <UpdateButton program={p} />
            </li>
          ))}
        </ul>
      )}
      <Link to="/">Back to Home</Link>
    </section>
  );
}
