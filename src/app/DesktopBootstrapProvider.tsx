import {
  createContext,
  type ReactNode,
  useEffect,
  useMemo,
  useState,
} from "react";
import {
  type DesktopBootstrapState,
  fetchBootstrapState,
  subscribeBootstrap,
} from "../api/desktop";

type DesktopBootstrapContextValue = {
  bootstrap: DesktopBootstrapState | null;
  error: string | null;
  loading: boolean;
};

const DesktopBootstrapContext = createContext<
  DesktopBootstrapContextValue | undefined
>(undefined);

export function DesktopBootstrapProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [bootstrap, setBootstrap] = useState<DesktopBootstrapState | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let mounted = true;
    let unsubscribe: (() => void) | undefined;

    const load = async () => {
      try {
        unsubscribe = await subscribeBootstrap((nextState) => {
          if (mounted) {
            setBootstrap(nextState);
            setLoading(false);
            setError(null);
          }
        });

        const initialState = await fetchBootstrapState();
        if (mounted) {
          setBootstrap(initialState);
          setLoading(false);
        }
      } catch (nextError) {
        if (mounted) {
          const message =
            nextError instanceof Error
              ? nextError.message
              : "Failed to load desktop bootstrap state.";
          setError(message);
          setLoading(false);
        }
      }
    };

    void load();

    return () => {
      mounted = false;
      unsubscribe?.();
    };
  }, []);

  const value = useMemo(
    () => ({
      bootstrap,
      error,
      loading,
    }),
    [bootstrap, error, loading],
  );

  return (
    <DesktopBootstrapContext.Provider value={value}>
      {children}
    </DesktopBootstrapContext.Provider>
  );
}

export { DesktopBootstrapContext };
