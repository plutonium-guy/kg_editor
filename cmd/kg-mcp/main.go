package main

import (
	"log/slog"
	"os"

	"github.com/mark3labs/mcp-go/server"

	mymcp "github.com/plutonium-guy/kg_editor/internal/mcp"
)

func main() {
	slog.SetDefault(slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelInfo})))

	base := os.Getenv("KG_SERVER_URL")
	if base == "" {
		base = "http://localhost:9000"
	}
	client := mymcp.NewClient(base)

	srv := server.NewMCPServer("kg-editor", "0.1.0")
	mymcp.RegisterTools(srv, client)

	if err := server.ServeStdio(srv); err != nil {
		slog.Error("kg-mcp stdio", "err", err)
		os.Exit(1)
	}
}
