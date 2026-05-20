package mcp

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestStdioCreateEntity(t *testing.T) {
	// Spin a tiny mock kg-server that returns a fixed entity on any request.
	mock := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("content-type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{"id": 42, "label": "Person", "props": map[string]any{"name": "Alice"}})
	}))
	defer mock.Close()

	// Locate repo root via this file's path.
	_, thisFile, _, _ := runtime.Caller(0)
	repo := filepath.Join(filepath.Dir(thisFile), "..", "..")

	// Build the binary.
	binPath := filepath.Join(repo, "bin", "kg-mcp")
	build := exec.Command("go", "build", "-o", binPath, "./cmd/kg-mcp")
	build.Dir = repo
	out, err := build.CombinedOutput()
	require.NoError(t, err, "build failed: %s", out)

	// Inherit PATH from the current environment.
	env := os.Environ()
	env = append(env, "KG_SERVER_URL="+mock.URL)

	cmd := exec.Command(binPath)
	cmd.Env = env
	stdin, err := cmd.StdinPipe()
	require.NoError(t, err)
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	require.NoError(t, cmd.Start())

	// JSON-RPC: initialize → initialized notification → tools/call.
	send := func(s string) { _, _ = stdin.Write([]byte(s + "\n")) }
	send(`{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}`)
	send(`{"jsonrpc":"2.0","method":"notifications/initialized"}`)
	send(`{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_entity","arguments":{"label":"Person","props":{"name":"Alice"}}}}`)

	// Wait briefly for responses, then kill.
	time.Sleep(500 * time.Millisecond)
	_ = cmd.Process.Kill()
	_ = cmd.Wait()

	got := stdout.String()
	assert.Contains(t, got, `"id":1`, "missing initialize response; stderr: %s", stderr.String())
	assert.True(t,
		strings.Contains(got, "Alice") || strings.Contains(got, "42"),
		"tool call response should contain mock data; got stdout: %s\nstderr: %s", got, stderr.String(),
	)
}
