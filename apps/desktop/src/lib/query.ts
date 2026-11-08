import { QueryClient } from "@tanstack/react-query";

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      // Data comes from the local core over IPC, not the network.
      queries: { retry: false, refetchOnWindowFocus: false, networkMode: "always" },
    },
  });
}
