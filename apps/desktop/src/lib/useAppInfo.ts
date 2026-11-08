import { useQuery } from "@tanstack/react-query";
import { getAppInfo } from "./ipc";

export function useAppInfo() {
  return useQuery({ queryKey: ["app-info"], queryFn: getAppInfo, staleTime: Infinity });
}
