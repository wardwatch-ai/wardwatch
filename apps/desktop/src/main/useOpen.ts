// Opening a screen that leads back to the one that opened it.
import { useLocation, useNavigate } from "react-router";

export interface From {
  from?: string;
}

/**
 * Opens a screen that leads back here: the fix and tune-up screens show
 * "Back to <this screen>" and return to it, scroll and state intact.
 */
export function useOpen() {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  return (path: string) => void navigate(path, { state: { from: pathname } satisfies From });
}
