// Settings' account card: sign in to the accounts server, pick a username, sign out.
// Shown only when the core has a server set (WARDWATCH_ACCOUNTS_URL); until then the
// app runs on sample data and this card stays hidden.
import { Button, Card, Eyebrow } from "@wardwatch/ui";
import { useQueryClient } from "@tanstack/react-query";
import { useId, useState, type SyntheticEvent } from "react";
import {
  accounts,
  useAccountStatus,
  useMe,
  useSignInState,
  type SignInState,
} from "../../lib/data";
import { accountsCancelSignIn, accountsSignIn, accountsSignOut } from "../../lib/ipc";

const inputClass = "min-h-11 rounded-[10px] border border-edge bg-surface px-3 text-text";
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

function SignIn() {
  const id = useId();
  const client = useQueryClient();
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  // Kept outside this card (App hears when the link is used or fails), so leaving
  // Settings while the link is on its way loses nothing.
  const { sentTo, error } = useSignInState().data ?? { sentTo: null, error: null };
  const set = (state: SignInState) => client.setQueryData(["sign-in"], state);

  const send = async (e: SyntheticEvent) => {
    e.preventDefault();
    setBusy(true);
    set({ sentTo: null, error: null });
    try {
      await accountsSignIn(email);
      set({ sentTo: email.trim(), error: null });
    } catch (err) {
      set({ sentTo: null, error: message(err) });
    } finally {
      setBusy(false);
    }
  };

  if (sentTo)
    return (
      <>
        <p className="m-0">
          We've emailed a link to <strong>{sentTo}</strong>. Open it on this device and press{" "}
          <strong>Confirm and sign in</strong> within 15 minutes. This page updates by itself.
        </p>
        <div>
          <Button
            variant="quiet"
            onClick={() => {
              void accountsCancelSignIn();
              set({ sentTo: null, error: null });
            }}
          >
            Use a different email
          </Button>
        </div>
      </>
    );

  return (
    <form onSubmit={(e) => void send(e)} className="flex flex-col gap-2">
      <p className="m-0">
        Sign in to keep your score and join groups. No password: we email you a link.
      </p>
      <label htmlFor={id} className="text-sm font-semibold">
        Email
      </label>
      <input
        id={id}
        type="email"
        autoComplete="email"
        value={email}
        onChange={(e) => {
          setEmail(e.target.value);
        }}
        className={inputClass}
        required
      />
      {error && (
        <p role="alert" className="m-0 text-sm">
          {error}
        </p>
      )}
      <div>
        <Button variant="brand" type="submit" disabled={busy}>
          Email me a sign-in link
        </Button>
      </div>
    </form>
  );
}

function PickUsername() {
  const id = useId();
  const client = useQueryClient();
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const save = async (e: SyntheticEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      await accounts.setUsername(name.trim());
      await client.invalidateQueries({ queryKey: ["me"] });
    } catch (err) {
      setError(message(err));
    } finally {
      setBusy(false);
    }
  };
  return (
    <form onSubmit={(e) => void save(e)} className="flex flex-col gap-2">
      <label htmlFor={id} className="text-sm font-semibold">
        Pick a username. Your groups see it next to your score.
      </label>
      <input
        id={id}
        value={name}
        onChange={(e) => {
          setName(e.target.value);
        }}
        className={inputClass}
        minLength={3}
        maxLength={20}
        pattern="[A-Za-z0-9_\-]+"
        required
      />
      {error && (
        <p role="alert" className="m-0 text-sm">
          {error}
        </p>
      )}
      <div>
        <Button variant="brand" type="submit" disabled={busy}>
          Save username
        </Button>
      </div>
    </form>
  );
}

function SignedIn() {
  const me = useMe().data;
  const [error, setError] = useState<string | null>(null);
  return (
    <>
      {me && me.username === null ? (
        <PickUsername />
      ) : (
        <p className="m-0">
          Signed in
          {me?.username && (
            <>
              {" "}
              as <strong>{me.username}</strong>
            </>
          )}
          .
        </p>
      )}
      {error && (
        <p role="alert" className="m-0 text-sm">
          {error}
        </p>
      )}
      <div>
        <Button
          variant="quiet"
          onClick={() => {
            accountsSignOut().catch((err: unknown) => {
              setError(message(err));
            });
          }}
        >
          Sign out of this device
        </Button>
      </div>
    </>
  );
}

export function AccountCard() {
  const status = useAccountStatus().data;
  if (!status?.configured) return null;
  return (
    <Card className="flex flex-col gap-3">
      <Eyebrow>Account</Eyebrow>
      {status.signed_in ? <SignedIn /> : <SignIn />}
    </Card>
  );
}
