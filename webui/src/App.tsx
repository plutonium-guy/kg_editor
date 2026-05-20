import styles from "./styles/app.module.css";

export default function App() {
  return (
    <div className={styles.app}>
      <header className={styles.toolbar}>kg editor — placeholder</header>
      <main className={styles.main}>
        <div className={styles.canvas}>Canvas placeholder</div>
        <aside className={styles.inspector}>Inspector placeholder</aside>
      </main>
      <footer className={styles.query}>Cypher box placeholder</footer>
    </div>
  );
}
