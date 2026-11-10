// React Query hooks the screens read from. Each source is cached under one key,
// so screens that show the same thing (Home, tray, Score) stay in step.
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { createHttpAccounts, createSampleAccounts, type AccountsClient } from "./accounts";
import {
  accountsRequest,
  accountsStatus,
  onAccountsChanged,
  getNudges,
  getPlan,
  getSettings,
  listAutoInstalls,
  listFindings,
  scanInventory,
  setPlan,
  setSettings,
  type Settings,
} from "./ipc";
import type { Period, UpdatePlan } from "./model";
import { buildPrograms, summarize } from "./programs";

const sampleAccounts = createSampleAccounts();
const httpAccounts = createHttpAccounts(accountsRequest);
let current: AccountsClient = sampleAccounts;

/**
 * The accounts client screens call: the real backend once this device is signed in
 * (useAccountsSource switches it), sample data otherwise.
 */
export const accounts: AccountsClient = {
  get preview() {
    return current.preview;
  },
  me: () => current.me(),
  setUsername: (username) => current.setUsername(username),
  score: () => current.score(),
  devices: () => current.devices(),
  groups: () => current.groups(),
  group: (id) => current.group(id),
  leaderboard: (id, period) => current.leaderboard(id, period),
  stakeIdeas: () => current.stakeIdeas(),
  setStake: (id, text) => current.setStake(id, text),
  setStakesEnabled: (id, enabled) => current.setStakesEnabled(id, enabled),
  createGroup: (name) => current.createGroup(name),
  joinGroup: (code) => current.joinGroup(code),
};

const ACCOUNT_KEYS = ["me", "score", "devices", "groups", "group", "leaderboard", "stake-ideas"];

/** Whether an accounts server is set up, and whether this device is signed in to it. */
export const useAccountStatus = () =>
  useQuery({ queryKey: ["account-status"], queryFn: accountsStatus, staleTime: Infinity });

/**
 * Points `accounts` at the backend while signed in and at the samples otherwise, and
 * refetches everything account-shaped when that changes. Mounted once, in App.
 */
export function useAccountsSource() {
  const client = useQueryClient();
  const signedIn = useAccountStatus().data?.signed_in ?? false;
  useEffect(
    () =>
      onAccountsChanged(() => {
        void client.invalidateQueries({ queryKey: ["account-status"] });
      }),
    [client],
  );
  useEffect(() => {
    const next = signedIn ? httpAccounts : sampleAccounts;
    if (next === current) return;
    current = next;
    for (const key of ACCOUNT_KEYS) void client.resetQueries({ queryKey: [key] });
  }, [signedIn, client]);
}

export function useInventory() {
  return useQuery({
    queryKey: ["inventory"],
    queryFn: () => scanInventory(),
    staleTime: 10 * 60_000,
  });
}

export function useFindings() {
  return useQuery({ queryKey: ["findings"], queryFn: listFindings, staleTime: 10 * 60_000 });
}

/** The nudge engine's plan. It reads the saved plan, so saving one refetches it. */
export function useNudges() {
  return useQuery({
    queryKey: ["nudges"],
    queryFn: getNudges,
    staleTime: 10 * 60_000,
  });
}

export function usePlan() {
  return useQuery({ queryKey: ["plan"], queryFn: getPlan, staleTime: Infinity });
}

export function useSavePlan() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: setPlan,
    // Show the change at once; roll back if the core refuses it.
    onMutate: async (plan: UpdatePlan) => {
      await client.cancelQueries({ queryKey: ["plan"] });
      const before = client.getQueryData<UpdatePlan>(["plan"]);
      client.setQueryData(["plan"], plan);
      return { before };
    },
    onError: (_e, _plan, ctx) => {
      if (ctx?.before) client.setQueryData(["plan"], ctx.before);
    },
    onSettled: () => client.invalidateQueries({ queryKey: ["nudges"] }),
  });
}

export function useAutoInstalls() {
  return useQuery({ queryKey: ["auto-installs"], queryFn: listAutoInstalls });
}

export function useSettings() {
  return useQuery({ queryKey: ["settings"], queryFn: getSettings, staleTime: Infinity });
}

export function useSaveSettings() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: setSettings,
    onSuccess: (_r, settings: Settings) => {
      client.setQueryData(["settings"], settings);
    },
  });
}

/** This device's programs and their summary. `programs` is null while loading. */
export function useDevice() {
  const inventory = useInventory();
  const findings = useFindings();
  const plan = usePlan();
  const nudges = useNudges();
  const holds = plan.data?.holds;
  const programs = useMemo(
    () =>
      inventory.data && findings.isSuccess && holds
        ? buildPrograms(inventory.data, findings.data, holds)
        : null,
    [inventory.data, findings.isSuccess, findings.data, holds],
  );
  const status = useMemo(
    () =>
      programs && !nudges.isPending
        ? summarize(programs, findings.data != null, nudges.data?.top_card ?? null)
        : null,
    [programs, findings.data, nudges.isPending, nudges.data],
  );
  return {
    inventory: inventory.data,
    programs,
    status,
    error: inventory.error ?? findings.error ?? plan.error,
    checkedAt: inventory.dataUpdatedAt,
  };
}

export const useMe = () => useQuery({ queryKey: ["me"], queryFn: () => accounts.me() });
export const useScore = () => useQuery({ queryKey: ["score"], queryFn: () => accounts.score() });
export const useDevices = () =>
  useQuery({ queryKey: ["devices"], queryFn: () => accounts.devices() });
export const useGroups = () => useQuery({ queryKey: ["groups"], queryFn: () => accounts.groups() });

export function useGroup(id: string | undefined) {
  return useQuery({
    queryKey: ["group", id],
    queryFn: () => accounts.group(id ?? ""),
    enabled: id !== undefined,
  });
}

export function useLeaderboard(id: string | undefined, period: Period) {
  return useQuery({
    queryKey: ["leaderboard", id, period],
    queryFn: () => accounts.leaderboard(id ?? "", period),
    enabled: id !== undefined,
  });
}

export const useStakeIdeas = (enabled: boolean) =>
  useQuery({
    queryKey: ["stake-ideas"],
    queryFn: () => accounts.stakeIdeas(),
    enabled,
    staleTime: Infinity,
  });

/** Wraps an accounts write and refreshes the group views afterwards. */
export function useGroupWrite<T>(write: (arg: T) => Promise<unknown>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: write,
    onSuccess: () =>
      Promise.all(
        ["groups", "group", "leaderboard"].map((k) => client.invalidateQueries({ queryKey: [k] })),
      ),
  });
}

/** Your place in your first group over the last 30 days, for Home and the tray. */
export function useGroupGlance() {
  const group = useGroups().data?.[0];
  const board = useLeaderboard(group?.id, "30d").data;
  if (!group || !board) return null;
  const you = board.members.find((m) => m.you);
  const leader = board.members[0];
  return { group, rank: you?.rank ?? null, leader: leader && !leader.you ? leader : null };
}
