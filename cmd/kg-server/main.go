package main

import (
	"context"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/plutonium-guy/kg_editor/internal/api"
	"github.com/plutonium-guy/kg_editor/internal/schema"
	"github.com/plutonium-guy/kg_editor/internal/schemastore"
	"github.com/plutonium-guy/kg_editor/internal/store"
)

func main() {
	slog.SetDefault(slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelInfo})))

	bind := envDefault("KG_SERVER_BIND", "127.0.0.1:9000")
	schemaPath := mustEnv("KG_SCHEMA")
	uri := envDefault("NEO4J_URI", "bolt://localhost:7687")
	user := envDefault("NEO4J_USER", "neo4j")
	password := mustEnv("NEO4J_PASSWORD")

	seed, err := schema.FromYAML(schemaPath)
	if err != nil {
		slog.Error("load seed schema", "err", err)
		os.Exit(1)
	}
	ctx := context.Background()
	st, err := store.New(ctx, uri, user, password)
	if err != nil {
		slog.Error("connect neo4j", "uri", uri, "err", err)
		os.Exit(1)
	}
	defer st.Close(ctx)

	ss := schemastore.New(st.Driver())
	current, err := ss.Bootstrap(ctx, seed)
	if err != nil {
		slog.Error("bootstrap schema", "err", err)
		os.Exit(1)
	}

	deps := api.NewDeps(current, st, ss)
	srv := &http.Server{
		Addr:              bind,
		Handler:           api.Router(deps),
		ReadHeaderTimeout: 5 * time.Second,
	}

	idleConnsClosed := make(chan struct{})
	go func() {
		sigint := make(chan os.Signal, 1)
		signal.Notify(sigint, os.Interrupt, syscall.SIGTERM)
		<-sigint
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := srv.Shutdown(shutdownCtx); err != nil {
			slog.Error("shutdown", "err", err)
		}
		close(idleConnsClosed)
	}()

	slog.Info("kg-server listening", "addr", bind, "schema", schemaPath, "neo4j", uri)
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		slog.Error("listen", "err", err)
		os.Exit(1)
	}
	<-idleConnsClosed
}

func envDefault(k, def string) string {
	if v, ok := os.LookupEnv(k); ok && v != "" {
		return v
	}
	return def
}

func mustEnv(k string) string {
	v := os.Getenv(k)
	if v == "" {
		slog.Error("required env var not set", "var", k)
		os.Exit(2)
	}
	return v
}
