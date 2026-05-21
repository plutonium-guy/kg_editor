// Package api exposes the HTTP surface for kg-server.
package api

import (
	"net/http"
	"sync/atomic"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"

	"github.com/plutonium-guy/kg_editor/internal/schema"
	"github.com/plutonium-guy/kg_editor/internal/schemastore"
	"github.com/plutonium-guy/kg_editor/internal/store"
)

// Deps is the set of dependencies the API handlers need.
type Deps struct {
	schema      atomic.Pointer[schema.File]
	Store       *store.Store
	SchemaStore *schemastore.Store
}

// NewDeps constructs a Deps with the initial schema pre-loaded into the atomic pointer.
func NewDeps(initial *schema.File, st *store.Store, ss *schemastore.Store) *Deps {
	d := &Deps{Store: st, SchemaStore: ss}
	d.schema.Store(initial)
	return d
}

// Schema returns the current cached schema (lock-free read).
func (d *Deps) Schema() *schema.File { return d.schema.Load() }

// SetSchema swaps the cached schema atomically. Called after any write to schemastore.
func (d *Deps) SetSchema(f *schema.File) { d.schema.Store(f) }

// Router returns a chi router with every endpoint registered.
func Router(d *Deps) http.Handler {
	r := chi.NewRouter()
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)
	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   []string{"*"},
		AllowedMethods:   []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"*"},
		AllowCredentials: false,
		MaxAge:           300,
	}))

	r.Get("/health", d.health)
	r.Get("/schema", d.getSchema)
	r.Put("/schema", d.putSchema)
	r.Post("/schema/reload", d.reloadSchema)
	r.Route("/schema/nodes", func(r chi.Router) {
		r.Post("/", d.createSchemaNode)
		r.Put("/{label}", d.updateSchemaNode)
		r.Delete("/{label}", d.deleteSchemaNode)
	})
	r.Route("/schema/rels", func(r chi.Router) {
		r.Post("/", d.createSchemaRel)
		r.Put("/{type}", d.updateSchemaRel)
		r.Delete("/{type}", d.deleteSchemaRel)
	})

	r.Route("/entities", func(r chi.Router) {
		r.Get("/", d.listEntities)
		r.Post("/", d.createEntity)
		r.Route("/{id}", func(r chi.Router) {
			r.Get("/", d.getEntity)
			r.Put("/", d.updateEntity)
			r.Delete("/", d.deleteEntity)
		})
	})

	r.Route("/links", func(r chi.Router) {
		r.Post("/", d.createLink)
		r.Delete("/{id}", d.deleteLink)
	})

	r.Get("/search", d.search)
	r.Post("/query", d.query)

	return r
}
