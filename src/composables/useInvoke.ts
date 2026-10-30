import { ref, shallowRef, type Ref } from "vue";
import { errMsg } from "@/lib/normalize";

export interface UseInvoke<T, A extends unknown[]> {
  data: Ref<T>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  run: (...args: A) => Promise<T>;
  reset: () => void;
}

/**
 * Wrap an async IPC call with loading/error state. Never throws: failures are
 * captured in `error` and `data` keeps its default value.
 * A monotonically increasing token drops stale responses.
 */
export function useInvoke<T, A extends unknown[] = []>(
  fn: (...args: A) => Promise<T>,
  initial: T,
): UseInvoke<T, A> {
  const data = shallowRef<T>(initial) as Ref<T>;
  const loading = ref(false);
  const error = ref<string | null>(null);
  let token = 0;

  async function run(...args: A): Promise<T> {
    const my = ++token;
    loading.value = true;
    error.value = null;
    try {
      const res = await fn(...args);
      if (my === token) data.value = res;
      return res;
    } catch (e) {
      if (my === token) {
        error.value = errMsg(e);
        data.value = initial;
      }
      return initial;
    } finally {
      if (my === token) loading.value = false;
    }
  }

  function reset() {
    token++;
    data.value = initial;
    loading.value = false;
    error.value = null;
  }

  return { data, loading, error, run, reset };
}
