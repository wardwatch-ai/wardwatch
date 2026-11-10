import { describe, expect, it, vi } from "vitest";
import { AccountsError, createHttpAccounts, type Call } from "./accounts";

const reply = (status: number, body: unknown) => Promise.resolve({ status, body });

describe("createHttpAccounts", () => {
  it("calls the backend's paths and maps its shapes to the screens'", async () => {
    const call = vi.fn<Call>((method, path) => {
      if (path === "/v1/groups") {
        return method === "GET"
          ? reply(200, {
              groups: [
                {
                  id: "g_1",
                  name: "Fam",
                  names: "username",
                  visibility: "private",
                  role: "owner",
                  member_count: 2,
                },
              ],
            })
          : reply(201, {
              id: "g_2",
              name: "New",
              names: "username",
              visibility: "private",
              member: true,
              role: "owner",
              invite_code: "abcdefghjk",
              stakes_enabled: true,
              stake: null,
              members: [{ user_id: "u_1" }],
              stake_ideas: [],
            });
      }
      if (path === "/v1/me")
        return reply(200, {
          id: "u_1",
          email: "a@b.c",
          username: "sam",
          real_name: null,
          username_changeable_at: null,
        });
      return reply(200, {});
    });
    const a = createHttpAccounts(call);
    expect(a.preview).toBe(false);
    expect(await a.me()).toEqual({ username: "sam", real_name: null });
    expect((await a.groups())[0]?.member_count).toBe(2);
    expect(await a.createGroup("New")).toEqual({
      id: "g_2",
      name: "New",
      names: "username",
      visibility: "private",
      role: "owner",
      member_count: 1,
    });
    await a.leaderboard("g/1", "month");
    await a.setStakesEnabled("g_1", false);
    await a.setUsername("sam_2");
    expect(call.mock.calls.slice(3)).toEqual([
      ["GET", "/v1/groups/g%2F1/leaderboard?period=month", undefined],
      ["PATCH", "/v1/groups/g_1", { stakes_enabled: false }],
      ["PATCH", "/v1/me", { username: "sam_2" }],
    ]);
    expect(await a.devices()).toEqual([]);
  });

  it("turns API errors into AccountsError with the backend's code and message", async () => {
    const a = createHttpAccounts(() =>
      reply(409, {
        error: { code: "username_required", message: "Pick a username before joining a group." },
      }),
    );
    const err = await a.joinGroup("x").catch((e: unknown) => e);
    expect(err).toBeInstanceOf(AccountsError);
    expect(err).toMatchObject({
      code: "username_required",
      message: "Pick a username before joining a group.",
    });

    const signedOut = createHttpAccounts(() => reply(401, null));
    await expect(signedOut.score()).rejects.toMatchObject({ code: "unauthorized" });
  });
});
