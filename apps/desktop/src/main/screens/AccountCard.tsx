// Settings' account card: sign in to the accounts server, pick a username, sign out.
// Shown only when the core has a server set (WARDWATCH_ACCOUNTS_URL); until then the
// app runs on sample data and this card stays hidden.
import { Button, Card, Eyebrow } from "@wardwatch/ui";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useState, type SyntheticEvent } from "react";
import { accounts, useAccountStatus, useMe } from "../../lib/data";
import {
  accountsCancelSignIn,
  accountsSignIn,
  accountsSignOut,
  onAccountsChanged,
} from "../../lib/ipc";

const inputClass = "min-h-11 rounded-[10px] border border-edge bg-surface px-3 text-text";
const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

function SignIn() {
  const id = useId();
  const [email, setEmail] = useState("");
  const [sentTo, setSentTo] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // The link was used (we're signed in and this card swaps out), or the wait failed.
  useEffect(
    () =>
      onAccountsChanged((e) => {
        if (e.error) {
          setSentTo(null);
          setError(e.error);
        }
      }),
    [],
  );

  const send = async (e: SyntheticEvent) => {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await accountsSignIn(email);
      setSentTo(email.trim());
    } catch (err) {
      setError(message(err));
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
              setSentTo(null);
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
  const [error, setError] = useState<string | null>(null);
  const save = async (e: SyntheticEvent) => {
    e.preventDefault();
    try {
      await accounts.setUsername(name.trim());
      await client.invalidateQueries({ queryKey: ["me"] });
    } catch (err) {
      setError(message(err));
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
        <Button variant="brand" type="submit">
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
