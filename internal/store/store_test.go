//go:build integration

package store

import (
	"context"
	"fmt"
	"os"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/wait"
)

func spinNeo4j(t *testing.T) (uri string, pw string, cleanup func()) {
	t.Helper()
	if envURI := os.Getenv("NEO4J_URI"); envURI != "" {
		return envURI, os.Getenv("NEO4J_PASSWORD"), func() {}
	}
	ctx := context.Background()
	req := testcontainers.ContainerRequest{
		Image:        "neo4j:5-community",
		ExposedPorts: []string{"7687/tcp"},
		Env:          map[string]string{"NEO4J_AUTH": "neo4j/testtest"},
		WaitingFor:   wait.ForLog("Started.").WithStartupTimeout(2 * time.Minute),
	}
	c, err := testcontainers.GenericContainer(ctx, testcontainers.GenericContainerRequest{ContainerRequest: req, Started: true})
	require.NoError(t, err)
	host, err := c.Host(ctx)
	require.NoError(t, err)
	port, err := c.MappedPort(ctx, "7687/tcp")
	require.NoError(t, err)
	uri = fmt.Sprintf("bolt://%s:%s", host, port.Port())
	return uri, "testtest", func() { _ = c.Terminate(ctx) }
}

func TestStoreSmoke(t *testing.T) {
	ctx := context.Background()
	uri, pw, cleanup := spinNeo4j(t)
	defer cleanup()
	s, err := New(ctx, uri, "neo4j", pw)
	require.NoError(t, err)
	defer s.Close(ctx)

	// create
	e, err := s.CreateNode(ctx, "Person", map[string]any{"name": "Alice", "role": "engineer"})
	require.NoError(t, err)
	assert.NotZero(t, e.ID)

	// get
	got, err := s.GetEntity(ctx, e.ID)
	require.NoError(t, err)
	assert.Equal(t, "Alice", got.Props["name"])

	// update
	require.NoError(t, s.UpdateNode(ctx, e.ID, map[string]any{"age": int64(31)}, []string{"role"}))

	// list
	lst, err := s.ListEntities(ctx, "Person", "", 50)
	require.NoError(t, err)
	assert.NotEmpty(t, lst)

	// create another + link
	e2, err := s.CreateNode(ctx, "Person", map[string]any{"name": "Bob"})
	require.NoError(t, err)
	rid, err := s.CreateLink(ctx, "KNOWS", e.ID, e2.ID, map[string]any{"since": int64(2024)})
	require.NoError(t, err)
	assert.NotZero(t, rid)

	// delete link
	require.NoError(t, s.DeleteLink(ctx, rid))

	// delete nodes
	require.NoError(t, s.DeleteNode(ctx, e.ID, true))
	require.NoError(t, s.DeleteNode(ctx, e2.ID, true))
}
