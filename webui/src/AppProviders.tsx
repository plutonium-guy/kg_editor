import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter } from "react-router-dom";
import type { ReactNode } from "react";

const qc = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
});

export default function AppProviders({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={qc}>
      <BrowserRouter>{children}</BrowserRouter>
    </QueryClientProvider>
  );
}
