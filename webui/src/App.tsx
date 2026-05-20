import { useCallback, useEffect, useRef, useState } from "react";
import styles from "./styles/app.module.css";
import Canvas from "./components/Canvas";
import Inspector from "./components/Inspector";
import Toolbar from "./components/Toolbar";
import QueryBox from "./components/QueryBox";
import ResultsTable from "./components/ResultsTable";
import { useStore } from "./state/store";
import { commit, runQuery } from "./kg/client";
import { refreshGraph } from "./kg/refresh";
import { JsUow, loadWasm } from "./kg/uow";

export default function App() {
  const [results, setResults] = useState<Record<string, unknown>[]>([]);
  const [error, setError] = useState<string | null>(null);
  const uowRef = useRef<JsUow | null>(null);
  const store = useStore();

  const ensureUow = useCallback((): JsUow => {
    if (!uowRef.current) uowRef.current = new JsUow();
    return uowRef.current;
  }, []);

  const handleCommit = useCallback(async () => {
    if (!uowRef.current) return;
    try {
      const out = uowRef.current.emit();
      await commit(out);
      uowRef.current = null;
      const g = await refreshGraph();
      store.replaceGraph(g.nodes, g.rels);
    } catch (e) {
      setError(String(e));
    }
  }, [store]);

  const handleDiscard = useCallback(() => {
    uowRef.current = null;
    store.resetPending();
  }, [store]);

  const handleRefresh = useCallback(async () => {
    try {
      const g = await refreshGraph();
      store.replaceGraph(g.nodes, g.rels);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [store]);

  const handleRunCypher = useCallback(async () => {
    // Defensive guard: phase-1 disables writes from the Cypher box.
    if (/\b(CREATE|MERGE|SET|DELETE|REMOVE|DROP)\b/i.test(store.cypherText)) {
      setError("Write operations from the Cypher box are disabled in phase 1; use the canvas editor.");
      return;
    }
    try {
      const res = await runQuery(store.cypherText);
      setResults(res.rows);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [store.cypherText]);

  useEffect(() => {
    (async () => {
      await loadWasm();
      await handleRefresh();
    })().catch((e) => setError(String(e)));
  }, [handleRefresh]);

  // Canvas dispatches kg-add-node / kg-add-rel; we handle here.
  useEffect(() => {
    const onAddNode = () => {
      const label = window.prompt("Node label? (e.g. Person)");
      if (!label) return;
      const name = window.prompt("Name?");
      const localId = ensureUow().createNode([label], name ? { name } : {});
      store.addPendingNode({ localId, labels: [label], props: name ? { name } : {} });
    };
    const onAddRel = (e: Event) => {
      const detail = (e as CustomEvent).detail as { from: string; to: string };
      const ty = window.prompt("Rel type? (e.g. KNOWS)");
      if (!ty) return;
      const refOf = (k: string): { kind: "server" | "local"; id: number } => ({
        kind: k.startsWith("s:") ? "server" : "local",
        id: Number(k.slice(2)),
      });
      const localRelId = ensureUow().createRel(refOf(detail.from), refOf(detail.to), ty, {});
      store.addPendingRel({
        localId: localRelId, type: ty,
        startKey: detail.from, endKey: detail.to, props: {},
      });
    };
    document.addEventListener("kg-add-node", onAddNode);
    document.addEventListener("kg-add-rel",  onAddRel);
    return () => {
      document.removeEventListener("kg-add-node", onAddNode);
      document.removeEventListener("kg-add-rel",  onAddRel);
    };
  }, [ensureUow, store]);

  return (
    <div className={styles.app}>
      <header className={styles.toolbar}>
        <Toolbar onCommit={handleCommit} onDiscard={handleDiscard} onRefresh={handleRefresh} />
      </header>
      <main className={styles.main}>
        <div className={styles.canvas}><Canvas /></div>
        <aside className={styles.inspector}>
          <Inspector
            onStageUpdateNode={(id, sets, unsets) => { ensureUow().updateNode(id, sets, unsets); store.setSelection({ kind: "none" }); }}
            onStageUpdateRel={(id, sets, unsets)  => { ensureUow().updateRel(id, sets, unsets);  store.setSelection({ kind: "none" }); }}
            onStageDeleteNode={(id) => { ensureUow().deleteNode(id, true); store.setSelection({ kind: "none" }); }}
            onStageDeleteRel={(id) =>  { ensureUow().deleteRel(id);        store.setSelection({ kind: "none" }); }}
          />
        </aside>
      </main>
      <footer className={styles.query}>
        <QueryBox onRun={handleRunCypher} />
        <ResultsTable rows={results} />
        {error && <div style={{ color: "red" }}>{error}</div>}
      </footer>
    </div>
  );
}
