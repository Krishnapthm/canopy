import { toast, type ExternalToast } from "sonner";

export function errorToast(error: unknown, options?: ExternalToast) {
  return toast.error(error instanceof Error ? error.message : String(error), {
    ...options, duration: Infinity,
  });
}
