//go:build integration

package api

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"github.com/plutonium-guy/kg_editor/internal/schema"
	"github.com/plutonium-guy/kg_editor/internal/store"
)

func setupServer(t *testing.T) (*httptest.Server, func()) {
	t.Helper()
	ctx := context.Background()
	uri := os.Getenv("NEO4J_URI")
	pw := os.Getenv("NEO4J_PASSWORD")
	if uri == "" || pw == "" {
		t.Skip("NEO4J_URI / NEO4J_PASSWORD not set; skipping integration test")
	}
	s, err := store.New(ctx, uri, "neo4j", pw)
	require.NoError(t, err)

	// Resolve repo-root kg-schema.yaml relative to this test file.
	yamlPath, err := filepath.Abs("../../kg-schema.yaml")
	require.NoError(t, err)
	sf, err := schema.FromYAML(yamlPath)
	require.NoError(t, err)

	srv := httptest.NewServer(Router(Deps{Schema: sf, Store: s}))
	return srv, func() {
		srv.Close()
		_ = s.Close(ctx)
	}
}

func postJSON(t *testing.T, url string, body any) *http.Response {
	t.Helper()
	b, _ := json.Marshal(body)
	r, err := http.Post(url, "application/json", bytes.NewReader(b))
	require.NoError(t, err)
	return r
}

func TestHealthAndSchema(t *testing.T) {
	srv, cleanup := setupServer(t)
	defer cleanup()

	r, err := http.Get(srv.URL + "/health")
	require.NoError(t, err)
	defer r.Body.Close()
	assert.Equal(t, http.StatusOK, r.StatusCode)
	var hv map[string]string
	require.NoError(t, json.NewDecoder(r.Body).Decode(&hv))
	assert.Equal(t, "ok", hv["status"])
	assert.Equal(t, "reachable", hv["neo4j"])

	r2, err := http.Get(srv.URL + "/schema")
	require.NoError(t, err)
	defer r2.Body.Close()
	var sv map[string]any
	require.NoError(t, json.NewDecoder(r2.Body).Decode(&sv))
	assert.NotNil(t, sv["nodes"])
	assert.NotNil(t, sv["rels"])
}

func TestEntityCRUD(t *testing.T) {
	srv, cleanup := setupServer(t)
	defer cleanup()

	// create
	r := postJSON(t, srv.URL+"/entities", map[string]any{
		"label": "Person",
		"props": map[string]any{"name": "Phil", "role": "engineer"},
	})
	require.Equal(t, http.StatusOK, r.StatusCode)
	var created map[string]any
	require.NoError(t, json.NewDecoder(r.Body).Decode(&created))
	r.Body.Close()
	idF, _ := created["id"].(float64)
	id := int64(idF)
	assert.NotZero(t, id)

	// get
	r2, err := http.Get(fmt.Sprintf("%s/entities/%d", srv.URL, id))
	require.NoError(t, err)
	defer r2.Body.Close()
	assert.Equal(t, http.StatusOK, r2.StatusCode)

	// update
	reqU, _ := http.NewRequest(http.MethodPut, fmt.Sprintf("%s/entities/%d", srv.URL, id), bytes.NewReader([]byte(`{"set":{"age":42},"unset":["role"]}`)))
	reqU.Header.Set("content-type", "application/json")
	r3, err := http.DefaultClient.Do(reqU)
	require.NoError(t, err)
	r3.Body.Close()
	assert.Equal(t, http.StatusOK, r3.StatusCode)

	// list
	r4, err := http.Get(srv.URL + "/entities?label=Person")
	require.NoError(t, err)
	defer r4.Body.Close()
	assert.Equal(t, http.StatusOK, r4.StatusCode)

	// delete
	reqD, _ := http.NewRequest(http.MethodDelete, fmt.Sprintf("%s/entities/%d?cascade=true", srv.URL, id), nil)
	r5, err := http.DefaultClient.Do(reqD)
	require.NoError(t, err)
	r5.Body.Close()
	assert.Equal(t, http.StatusOK, r5.StatusCode)
}

func TestValidationErrors(t *testing.T) {
	srv, cleanup := setupServer(t)
	defer cleanup()

	// missing required name
	r := postJSON(t, srv.URL+"/entities", map[string]any{"label": "Person", "props": map[string]any{"role": "engineer"}})
	defer r.Body.Close()
	assert.Equal(t, http.StatusBadRequest, r.StatusCode)

	// bad enum
	r2 := postJSON(t, srv.URL+"/entities", map[string]any{"label": "Person", "props": map[string]any{"name": "X", "role": "wizard"}})
	defer r2.Body.Close()
	assert.Equal(t, http.StatusBadRequest, r2.StatusCode)
}

func TestLinkCRUDAndSearch(t *testing.T) {
	srv, cleanup := setupServer(t)
	defer cleanup()

	r1 := postJSON(t, srv.URL+"/entities", map[string]any{"label": "Person", "props": map[string]any{"name": "Gina"}})
	defer r1.Body.Close()
	var v1 map[string]any
	_ = json.NewDecoder(r1.Body).Decode(&v1)
	g := int64(v1["id"].(float64))

	r2 := postJSON(t, srv.URL+"/entities", map[string]any{"label": "Person", "props": map[string]any{"name": "Hank"}})
	defer r2.Body.Close()
	var v2 map[string]any
	_ = json.NewDecoder(r2.Body).Decode(&v2)
	h := int64(v2["id"].(float64))

	// create link
	r3 := postJSON(t, srv.URL+"/links", map[string]any{"type": "KNOWS", "start_id": g, "end_id": h, "props": map[string]any{"since": 2024}})
	defer r3.Body.Close()
	assert.Equal(t, http.StatusOK, r3.StatusCode)
	var lv map[string]any
	_ = json.NewDecoder(r3.Body).Decode(&lv)
	rid := int64(lv["id"].(float64))
	assert.NotZero(t, rid)

	// search finds Gina
	r4, err := http.Get(srv.URL + "/search?q=Gina")
	require.NoError(t, err)
	defer r4.Body.Close()
	var sr []map[string]any
	_ = json.NewDecoder(r4.Body).Decode(&sr)
	assert.NotEmpty(t, sr)

	// delete link
	reqD, _ := http.NewRequest(http.MethodDelete, fmt.Sprintf("%s/links/%d", srv.URL, rid), nil)
	r5, err := http.DefaultClient.Do(reqD)
	require.NoError(t, err)
	r5.Body.Close()
	assert.Equal(t, http.StatusOK, r5.StatusCode)
}
