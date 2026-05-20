// Package api exposes the HTTP surface for kg-server.
package api

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"

	"github.com/plutonium-guy/kg_editor/internal/schema"
	"github.com/plutonium-guy/kg_editor/internal/store"
)

// Deps is the set of dependencies the API handlers need.
type Deps struct {
	Schema *schema.File
	Store  *store.Store
}

// Router returns a chi router with every endpoint registered.
func Router(d Deps) http.Handler {
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

	return r
}
